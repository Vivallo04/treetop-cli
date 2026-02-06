use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    Animate,
    Resize,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<Event>();

        let task = tokio::spawn(async move {
            let mut reader = event::EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);
            let mut animate_interval = tokio::time::interval(Duration::from_millis(40));

            loop {
                tokio::select! {
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                let mapped = match evt {
                                    CrosstermEvent::Key(key) => Some(Event::Key(key)),
                                    CrosstermEvent::Mouse(mouse) => Some(Event::Mouse(mouse)),
                                    CrosstermEvent::Resize(_, _) => Some(Event::Resize),
                                    _ => None,
                                };
                                if let Some(e) = mapped
                                    && tx.send(e).is_err()
                                {
                                    break;
                                }
                            }
                            Some(Err(_)) => break,
                            None => break,
                        }
                    }
                    _ = tick_interval.tick() => {
                        if tx.send(Event::Tick).is_err() {
                            break;
                        }
                    }
                    _ = animate_interval.tick() => {
                        if tx.send(Event::Animate).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, _task: task }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
