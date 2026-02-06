use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(
            " q",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit  "),
        Span::styled(
            "\u{2190}\u{2191}\u{2192}\u{2193}",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Navigate"),
    ]);
    let style = Style::default().bg(Color::DarkGray);
    frame.render_widget(Paragraph::new(line).style(style), area);
}
