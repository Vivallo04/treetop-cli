mod app;
mod event;
mod system;
mod treemap;
mod ui;

use std::time::Duration;

use app::App;
use color_eyre::Result;
use event::{Event, EventHandler};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = ratatui::init();

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    let result = run(&mut terminal).await;

    ratatui::restore();

    result
}

async fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    let mut events = EventHandler::new(Duration::from_secs(2));

    terminal.draw(|frame| ui::draw(frame, &mut app))?;

    while app.running {
        if let Some(event) = events.next().await {
            match event {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        app.handle_key(key);
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
