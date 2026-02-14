use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::format::{format_bytes, truncate_unicode};
use crate::ui::theme::Theme;

#[derive(Debug, Clone)]
pub struct SelectionInfo {
    pub name: String,
    pub memory_bytes: u64,
}

pub fn render(frame: &mut Frame, area: Rect, selected: Option<SelectionInfo>, theme: &Theme) {
    let style = Style::default()
        .bg(theme.statusbar_bg)
        .fg(theme.text_primary);
    let width = area.width as usize;
    let line = match selected {
        Some(selection) => format_selection_line(selection, width),
        None => " ".repeat(width),
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(line, style))).style(style),
        area,
    );
}

fn format_selection_line(selection: SelectionInfo, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let mut memory = format_bytes(selection.memory_bytes);
    if memory.width() > width {
        memory = truncate_unicode(&memory, width);
        let pad = width.saturating_sub(memory.width());
        return format!("{}{}", " ".repeat(pad), memory);
    }

    let memory_width = memory.width();
    let left_capacity = width.saturating_sub(memory_width + 1);
    let name = truncate_unicode(&selection.name, left_capacity);
    let name_width = name.width();
    let gap = width.saturating_sub(name_width + memory_width);
    format!("{name}{}{memory}", " ".repeat(gap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_memory_right_aligned() {
        let line = format_selection_line(
            SelectionInfo {
                name: "Very Long Process Name".to_string(),
                memory_bytes: 1_234_567_890,
            },
            24,
        );
        assert!(line.ends_with("1.1 GB"));
    }
}
