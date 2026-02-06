use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::InputMode;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    input_mode: InputMode,
    filter_text: &str,
    status_message: Option<&(String, std::time::Instant)>,
) {
    let bg_style = Style::default().bg(Color::DarkGray);

    // Status message takes priority
    if let Some((msg, _)) = status_message {
        let color = if msg.starts_with("Sent") || msg.starts_with("Killed") {
            Color::Green
        } else {
            Color::Red
        };
        let line = Line::from(Span::styled(
            format!(" {msg}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(line).style(bg_style), area);
        return;
    }

    let line = match input_mode {
        InputMode::Filter => {
            Line::from(vec![
                Span::styled(
                    " / ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" {filter_text}")),
                Span::styled(
                    "\u{2588}",
                    Style::default().fg(Color::Yellow),
                ),
            ])
        }
        InputMode::Normal if !filter_text.is_empty() => {
            Line::from(vec![
                Span::styled(
                    " Filter: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(filter_text),
                Span::raw("  "),
                key_span("Esc"),
                Span::raw(" Clear  "),
                key_span("/"),
                Span::raw(" Edit"),
            ])
        }
        InputMode::Normal => {
            Line::from(vec![
                key_span(" q"),
                Span::raw(" Quit  "),
                key_span("/"),
                Span::raw(" Filter  "),
                key_span("k"),
                Span::raw(" Kill  "),
                key_span("d"),
                Span::raw(" Detail  "),
                key_span("c"),
                Span::raw(" Color  "),
                key_span("\u{2190}\u{2191}\u{2192}\u{2193}"),
                Span::raw(" Navigate"),
            ])
        }
    };

    frame.render_widget(Paragraph::new(line).style(bg_style), area);
}

fn key_span(key: &str) -> Span<'_> {
    Span::styled(
        key,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
}
