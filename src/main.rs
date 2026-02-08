mod action;
mod app;
mod config;
mod event;
mod format;
#[cfg(feature = "perf-tracing")]
mod perf;
mod system;
mod treemap;
mod ui;

use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use app::App;
use clap::Parser;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use config::{load_config, load_config_from_path};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, MouseEventKind};
use crossterm::execute;
use event::{Event, EventHandler};

#[derive(Parser)]
#[command(
    name = "treetop",
    about = "TUI system monitor with treemap visualization"
)]
struct Cli {
    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Refresh rate in milliseconds
    #[arg(long)]
    refresh_rate: Option<u64>,

    /// Color mode: name, memory, cpu, user, group, mono
    #[arg(long)]
    color_mode: Option<String>,

    /// Color support: auto, 256, truecolor, mono
    #[arg(long)]
    color: Option<String>,

    /// Run headless performance capture without interactive terminal.
    #[arg(long, default_value_t = false)]
    perf_capture: bool,

    /// Number of capture iterations for perf mode.
    #[arg(long, default_value_t = 120)]
    perf_iterations: usize,

    /// Headless terminal width for perf mode.
    #[arg(long, default_value_t = 160)]
    perf_width: u16,

    /// Headless terminal height for perf mode.
    #[arg(long, default_value_t = 50)]
    perf_height: u16,

    /// Perf tracing output file (JSON lines).
    #[arg(long, default_value = "target/perf/perf_spans.jsonl")]
    perf_output: PathBuf,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let config = load_config_for_cli(&cli);

    if cli.perf_capture {
        return run_perf_capture(config, &cli);
    }

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        ratatui::restore();
        original_hook(panic_info);
    }));

    let result = run(&mut terminal, config).await;

    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();

    result
}

async fn run(terminal: &mut ratatui::DefaultTerminal, config: config::Config) -> Result<()> {
    let tick_rate = Duration::from_millis(config.general.refresh_rate_ms);
    let mut app = App::new(config);
    let mut events = EventHandler::new(tick_rate);

    terminal.draw(|frame| ui::draw(frame, &mut app))?;

    while app.running {
        if let Some(event) = events.next().await {
            let mut should_draw = false;
            match event {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        let action = app.map_key(key);
                        app.dispatch(action);
                        should_draw = true;
                    }
                }
                Event::Mouse(mouse) => {
                    if mouse.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                        let action = action::Action::SelectAt(mouse.column, mouse.row);
                        app.dispatch(action);
                        should_draw = true;
                    }
                }
                Event::Tick => {
                    app.refresh_data();
                    should_draw = true;
                }
                Event::Animate => {
                    if app.is_animating() {
                        app.tick_animation();
                        should_draw = true;
                    }
                }
                Event::Resize => {
                    app.on_resize();
                    should_draw = true;
                }
            }
            if should_draw {
                terminal.draw(|frame| ui::draw(frame, &mut app))?;
            }
        }
    }

    Ok(())
}

fn load_config_for_cli(cli: &Cli) -> config::Config {
    let mut config = match &cli.config {
        Some(path) => load_config_from_path(path),
        None => load_config(),
    };

    if let Some(rate) = cli.refresh_rate {
        config.general.refresh_rate_ms = rate;
    }
    if let Some(ref mode) = cli.color_mode {
        config.general.default_color_mode = mode.clone();
    }
    if let Some(ref support) = cli.color {
        config.general.color_support = support.clone();
    }

    config
}

fn run_perf_capture(config: config::Config, cli: &Cli) -> Result<()> {
    #[cfg(not(feature = "perf-tracing"))]
    {
        let _ = (config, cli);
        Err(eyre!(
            "--perf-capture requires the `perf-tracing` feature; run with `cargo run --features perf-tracing -- --perf-capture`"
        ))
    }

    #[cfg(feature = "perf-tracing")]
    {
        if cli.perf_iterations == 0 {
            return Err(eyre!("--perf-iterations must be greater than 0"));
        }
        if cli.perf_width == 0 || cli.perf_height == 0 {
            return Err(eyre!(
                "--perf-width and --perf-height must be greater than 0"
            ));
        }

        if cli.perf_output.exists() {
            std::fs::remove_file(&cli.perf_output)?;
        }
        perf::init_tracing_json(&cli.perf_output)?;

        let mut app = App::new(config);
        let backend = ratatui::backend::TestBackend::new(cli.perf_width, cli.perf_height);
        let mut terminal = ratatui::Terminal::new(backend)?;
        let mut process_counts = Vec::with_capacity(cli.perf_iterations);

        for _ in 0..cli.perf_iterations {
            app.refresh_data();
            process_counts.push(app.snapshot.process_tree.processes.len());
            terminal.draw(|frame| ui::draw(frame, &mut app))?;
        }

        perf::write_baseline_artifacts(
            &cli.perf_output,
            cli.perf_iterations,
            cli.perf_width,
            cli.perf_height,
            &process_counts,
        )?;

        println!("Perf baseline updated:");
        println!(" - docs/perf_baseline.json");
        println!(" - docs/PERF_BASELINE.md");
        println!(" - {}", cli.perf_output.display());
        Ok(())
    }
}
