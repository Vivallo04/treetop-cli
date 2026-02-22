pub mod detail_panel;
pub mod header;
pub mod help;
pub mod selection_bar;
pub mod statusbar;
pub mod theme;
pub mod treemap_widget;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;
use crate::ui::theme::colorize_rects;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(1),
            Constraint::Length(1),
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

        let rects = app.display_rects();
        let colored = colorize_rects(
            &rects,
            &app.snapshot.process_tree,
            app.snapshot.memory_total,
            app.color_mode,
            &app.theme,
            app.color_support,
        );
        treemap_widget::render(
            frame,
            treemap_area,
            &colored,
            app.selected_index,
            app.min_rect_width,
            app.min_rect_height,
            app.border_style,
            &app.theme,
        );

        if let Some(process) = app.selected_process() {
            let history = app.history.get(process.pid);
            detail_panel::render(
                frame,
                detail_area,
                process,
                &app.theme,
                app.border_style,
                history,
            );
        }
    } else {
        app.treemap_area = Some(content_area);
        app.compute_layout(content_area.width, content_area.height);
        let rects = app.display_rects();
        let colored = colorize_rects(
            &rects,
            &app.snapshot.process_tree,
            app.snapshot.memory_total,
            app.color_mode,
            &app.theme,
            app.color_support,
        );
        treemap_widget::render(
            frame,
            content_area,
            &colored,
            app.selected_index,
            app.min_rect_width,
            app.min_rect_height,
            app.border_style,
            &app.theme,
        );
    }

    let breadcrumbs = app.zoom_breadcrumbs();
    header::render(
        frame,
        chunks[0],
        &app.snapshot,
        app.color_mode,
        &app.theme,
        app.border_style,
        &breadcrumbs,
        &app.cpu_history,
    );
    statusbar::render(
        frame,
        chunks[3],
        app.input_mode,
        &app.filter_text,
        app.status_message.as_ref(),
        &app.theme,
        app.is_zoomed(),
    );

    let selected = app
        .selected_process()
        .map(|p| selection_bar::SelectionInfo {
            pid: p.pid,
            name: p.name.clone(),
            memory_bytes: p.memory_bytes,
        });
    selection_bar::render(frame, chunks[2], selected, &app.theme);

    // Help overlay — rendered last to appear on top
    if app.show_help() {
        help::render(frame, frame.area(), &app.help_entries(), &app.theme);
    }
}

#[cfg(test)]
mod tests;
