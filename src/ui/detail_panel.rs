use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::system::process::ProcessInfo;

pub fn render(frame: &mut Frame, area: Rect, process: &ProcessInfo) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            " Process Detail ",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

    let mem_str = format_bytes(process.memory_bytes);
    let cmd_display = if process.command.len() > 60 {
        format!("{}...", &process.command[..57])
    } else if process.command.is_empty() {
        "(none)".to_string()
    } else {
        process.command.clone()
    };

    let lines = vec![
        detail_line("PID", process.pid.to_string()),
        detail_line("PPID", process.ppid.to_string()),
        detail_line("Name", process.name.clone()),
        detail_line("Cmd", cmd_display),
        detail_line("Memory", mem_str),
        detail_line("CPU", format!("{:.1}%", process.cpu_percent)),
        detail_line(
            "User",
            process
                .user_id
                .as_deref()
                .unwrap_or("N/A")
                .to_string(),
        ),
        detail_line(
            "Group",
            process
                .group_id
                .as_deref()
                .unwrap_or("N/A")
                .to_string(),
        ),
        detail_line("Status", process.status.clone()),
        detail_line("Children", process.children.len().to_string()),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn detail_line(label: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {label:<9}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
