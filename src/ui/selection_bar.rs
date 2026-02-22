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
    pub pid: u32,
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

    let pid_prefix = format!("[{}] ", selection.pid);
    let mut memory = format_bytes(selection.memory_bytes);
    if memory.width() > width {
        memory = truncate_unicode(&memory, width);
        let pad = width.saturating_sub(memory.width());
        return format!("{}{}", " ".repeat(pad), memory);
    }

    let memory_width = memory.width();
    let left_capacity = width.saturating_sub(memory_width + 1);

    // Try to fit pid prefix + name
    let pid_width = pid_prefix.width();
    if pid_width >= left_capacity {
        // Not enough room for pid prefix, just show name
        let name = truncate_unicode(&selection.name, left_capacity);
        let name_width = name.width();
        let gap = width.saturating_sub(name_width + memory_width);
        return format!("{name}{}{memory}", " ".repeat(gap));
    }

    let name_capacity = left_capacity - pid_width;
    let name = truncate_unicode(&selection.name, name_capacity);
    let label = format!("{pid_prefix}{name}");
    let label_width = label.width();
    let gap = width.saturating_sub(label_width + memory_width);
    format!("{label}{}{memory}", " ".repeat(gap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_memory_right_aligned() {
        let line = format_selection_line(
            SelectionInfo {
                pid: 1234,
                name: "Very Long Process Name".to_string(),
                memory_bytes: 1_234_567_890,
            },
            30,
        );
        assert!(line.ends_with("1.1 GB"));
    }

    #[test]
    fn pid_prefix_shown() {
        let line = format_selection_line(
            SelectionInfo {
                pid: 42,
                name: "firefox".to_string(),
                memory_bytes: 512_000_000,
            },
            40,
        );
        assert!(line.starts_with("[42] firefox"));
        assert!(line.ends_with("488.3 MB"));
    }
}
