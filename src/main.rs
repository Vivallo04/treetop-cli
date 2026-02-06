mod action;
mod app;
mod config;
mod event;
mod system;
mod treemap;
mod ui;

use std::io::stdout;
use std::path::PathBuf;
use std::time::Duration;

use app::App;
use clap::Parser;
use color_eyre::Result;
use config::{load_config, load_config_from_path};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, MouseEventKind};
use crossterm::execute;
use event::{Event, EventHandler};

#[derive(Parser)]
#[command(name = "treetop", about = "TUI system monitor with treemap visualization")]
struct Cli {
    /// Path to config file
    #[arg(long)]
    config: Option<PathBuf>,

    /// Refresh rate in milliseconds
    #[arg(long)]
    refresh_rate: Option<u64>,

    /// Color mode: memory, cpu, user, group, mono
    #[arg(long)]
    color_mode: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        ratatui::restore();
        original_hook(panic_info);
    }));

    let result = run(&mut terminal, cli).await;

    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();

    result
}

async fn run(terminal: &mut ratatui::DefaultTerminal, cli: Cli) -> Result<()> {
    let mut config = match &cli.config {
        Some(path) => load_config_from_path(path),
        None => load_config(),
    };

    // CLI overrides
    if let Some(rate) = cli.refresh_rate {
        config.general.refresh_rate_ms = rate;
    }
    if let Some(ref mode) = cli.color_mode {
        config.general.default_color_mode = mode.clone();
    }

    let tick_rate = Duration::from_millis(config.general.refresh_rate_ms);
    let mut app = App::new(config);
    let mut events = EventHandler::new(tick_rate);

    terminal.draw(|frame| ui::draw(frame, &mut app))?;

    while app.running {
        if let Some(event) = events.next().await {
            match event {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        let action = app.map_key(key);
                        app.dispatch(action);
                    }
                }
                Event::Mouse(mouse) => {
                    if mouse.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                        let action =
                            action::Action::SelectAt(mouse.column, mouse.row);
                        app.dispatch(action);
                    }
                }
                Event::Tick => {
                    app.refresh_data();
                }
                Event::Resize(_, _) => {
                    app.on_resize();
                }
            }
        }

        terminal.draw(|frame| ui::draw(frame, &mut app))?;
    }

    Ok(())
}
