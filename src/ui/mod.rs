pub mod header;
pub mod statusbar;
pub mod treemap_widget;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    app.compute_layout(chunks[1].width, chunks[1].height);

    header::render(frame, chunks[0], &app.snapshot);
    treemap_widget::render(frame, chunks[1], &app.layout_rects, app.selected_index);
    statusbar::render(frame, chunks[2]);
}
