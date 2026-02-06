use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::InputMode;
use crate::ui::theme::Theme;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    input_mode: InputMode,
    filter_text: &str,
    status_message: Option<&(String, std::time::Instant)>,
    theme: &Theme,
    is_zoomed: bool,
) {
    let bg_style = Style::default().bg(theme.statusbar_bg);

    // Status message takes priority
    if let Some((msg, _)) = status_message {
        let color = if msg.starts_with("Sent") || msg.starts_with("Killed") {
            theme.status_ok
        } else {
            theme.status_err
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
            let mut spans = vec![
                Span::styled(
                    " / ",
                    Style::default()
                        .fg(theme.pill_key_fg)
                        .bg(theme.pill_key_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {filter_text}"),
                    Style::default().fg(theme.pill_desc_fg),
                ),
                Span::styled("\u{2588}", Style::default().fg(theme.pill_key_bg)),
            ];
            spans.extend(pill_spans("Esc", "Cancel", theme));
            spans.extend(pill_spans("Enter", "Apply", theme));
            Line::from(spans)
        }
        InputMode::Normal if !filter_text.is_empty() => {
            let mut spans = vec![
                Span::styled(
                    " Filter: ",
                    Style::default()
                        .fg(theme.pill_key_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(filter_text, Style::default().fg(theme.pill_desc_fg)),
            ];
            spans.extend(pill_spans("Esc", "Clear", theme));
            spans.extend(pill_spans("/", "Edit", theme));
            Line::from(spans)
        }
        InputMode::Normal => {
            let mut spans = Vec::new();
            spans.extend(pill_spans("q", "Quit", theme));
            spans.extend(pill_spans("/", "Filter", theme));
            spans.extend(pill_spans("Enter", "Zoom", theme));
            if is_zoomed {
                spans.extend(pill_spans("Esc", "Back", theme));
            }
            spans.extend(pill_spans("k", "Kill", theme));
            spans.extend(pill_spans("d", "Detail", theme));
            spans.extend(pill_spans("c", "Color", theme));
            spans.extend(pill_spans("t", "Theme", theme));
            spans.extend(pill_spans("\u{2190}\u{2193}\u{2191}\u{2192}", "Nav", theme));
            Line::from(spans)
        }
    };

    frame.render_widget(Paragraph::new(line).style(bg_style), area);
}

fn pill_spans<'a>(key: &'a str, desc: &'a str, theme: &Theme) -> Vec<Span<'a>> {
    vec![
        Span::raw(" "),
        Span::styled(
            format!(" {key} "),
            Style::default()
                .fg(theme.pill_key_fg)
                .bg(theme.pill_key_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {desc}"),
            Style::default().fg(theme.pill_desc_fg).bg(theme.surface_bg),
        ),
    ]
}
