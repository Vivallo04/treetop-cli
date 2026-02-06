use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Sparkline};
use ratatui::Frame;

use crate::system::history::ProcessHistory;
use crate::system::process::ProcessInfo;
use crate::treemap::color::Theme;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    process: &ProcessInfo,
    theme: &Theme,
    history: Option<&ProcessHistory>,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.overlay_border))
        .title(Span::styled(
            " Process Detail ",
            Style::default()
                .fg(theme.accent_mauve)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner into info section and sparkline section
    let has_history = history.is_some_and(|h| h.memory.len() > 1);
    let chunks = if has_history && inner.height > 14 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Min(4)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(0)])
            .split(inner)
    };

    let mem_str = format_bytes(process.memory_bytes);
    let cmd_display = if process.command.len() > 60 {
        format!("{}...", &process.command[..57])
    } else if process.command.is_empty() {
        "(none)".to_string()
    } else {
        process.command.clone()
    };

    let lines = vec![
        detail_line("PID", process.pid.to_string(), theme),
        detail_line("PPID", process.ppid.to_string(), theme),
        detail_line("Name", process.name.clone(), theme),
        detail_line("Cmd", cmd_display, theme),
        detail_line("Memory", mem_str, theme),
        detail_line("CPU", format!("{:.1}%", process.cpu_percent), theme),
        detail_line(
            "User",
            process
                .user_id
                .as_deref()
                .unwrap_or("N/A")
                .to_string(),
            theme,
        ),
        detail_line(
            "Group",
            process
                .group_id
                .as_deref()
                .unwrap_or("N/A")
                .to_string(),
            theme,
        ),
        detail_line("Status", process.status.clone(), theme),
        detail_line("Children", process.children.len().to_string(), theme),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, chunks[0]);

    // Render sparklines if we have history data and enough space
    if let Some(hist) = history
        && hist.memory.len() > 1
        && chunks[1].height >= 4
    {
            let spark_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(chunks[1]);

            // Memory sparkline
            let mem_data: Vec<u64> = hist.memory.iter().copied().collect();
            let mem_spark = Sparkline::default()
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(theme.overlay_border))
                        .title(Span::styled(
                            " Memory ",
                            Style::default()
                                .fg(theme.accent_mauve)
                                .add_modifier(Modifier::BOLD),
                        )),
                )
                .data(&mem_data)
                .style(Style::default().fg(theme.gauge_filled));
            frame.render_widget(mem_spark, spark_chunks[0]);

            // CPU sparkline (convert f32 percentage to u64, scale by 100 for precision)
            let cpu_data: Vec<u64> = hist.cpu.iter().map(|&c| (c * 100.0) as u64).collect();
            let cpu_spark = Sparkline::default()
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(theme.overlay_border))
                        .title(Span::styled(
                            " CPU ",
                            Style::default()
                                .fg(theme.accent_mauve)
                                .add_modifier(Modifier::BOLD),
                        )),
                )
                .data(&cpu_data)
                .max(10000) // 100.00%
                .style(Style::default().fg(theme.sparkline_color));
            frame.render_widget(cpu_spark, spark_chunks[1]);
    }
}

fn detail_line(label: &str, value: String, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {label:<9}"),
            Style::default()
                .fg(theme.accent_mauve)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, Style::default().fg(theme.text_primary)),
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
