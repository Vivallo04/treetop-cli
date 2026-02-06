pub mod detail_panel;
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

    let content_area = chunks[1];

    if app.show_detail_panel {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(20), Constraint::Length(35)])
            .split(content_area);

        let treemap_area = h_chunks[0];
        let detail_area = h_chunks[1];

        app.treemap_area = Some(treemap_area);
        app.compute_layout(treemap_area.width, treemap_area.height);

        treemap_widget::render(frame, treemap_area, &app.layout_rects, app.selected_index, app.min_rect_width, app.min_rect_height);

        if let Some(process) = app.selected_process() {
            detail_panel::render(frame, detail_area, process);
        }
    } else {
        app.treemap_area = Some(content_area);
        app.compute_layout(content_area.width, content_area.height);
        treemap_widget::render(frame, content_area, &app.layout_rects, app.selected_index, app.min_rect_width, app.min_rect_height);
    }

    header::render(frame, chunks[0], &app.snapshot, app.color_mode);
    statusbar::render(
        frame,
        chunks[2],
        app.input_mode,
        &app.filter_text,
        app.status_message.as_ref(),
    );
}
