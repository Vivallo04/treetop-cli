use ratatui::Frame;
use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::ui::theme::Theme;

/// Renders a centered help overlay with all keybind → description pairs.
pub fn render(frame: &mut Frame, area: Rect, entries: &[(String, &str)], theme: &Theme) {
    let width = 40u16.min(area.width.saturating_sub(4));
    let height = (entries.len() as u16 + 2).min(area.height.saturating_sub(2)); // +2 for borders

    let overlay = centered_rect(width, height, area);

    // Clear the area behind the overlay
    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.overlay_border))
        .title(Span::styled(
            " Keybinds ",
            Style::default()
                .fg(theme.accent_mauve)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(overlay);

    let lines: Vec<Line> = entries
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!(" {key:>8} ", key = key),
                    Style::default()
                        .fg(theme.pill_key_fg)
                        .bg(theme.pill_key_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {desc}"), Style::default().fg(theme.pill_desc_fg)),
            ])
        })
        .collect();

    frame.render_widget(block, overlay);
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.surface_bg)),
        inner,
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let [vert] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [horiz] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(vert);
    horiz
}
