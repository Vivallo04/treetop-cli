use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};

use crate::format::{format_bytes, truncate_unicode};
use crate::system::history::ProcessHistory;
use crate::system::process::ProcessInfo;
use crate::ui::theme::{BorderStyle, Theme};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    process: &ProcessInfo,
    theme: &Theme,
    border_style: BorderStyle,
    history: Option<&ProcessHistory>,
) {
    let borders = if border_style.has_border() {
        Borders::ALL
    } else {
        Borders::NONE
    };
    let block = Block::default()
        .borders(borders)
        .border_type(border_style.border_type())
        .border_style(Style::default().fg(theme.overlay_border))
        .title(Span::styled(
            " Process Detail ",
            Style::default()
                .fg(theme.accent_mauve)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let has_history = history.is_some_and(|h| h.memory.len() > 1);
    let chunks = if has_history && inner.height > 14 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(13), Constraint::Min(4)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(0)])
            .split(inner)
    };

    let mem_str = format_bytes(process.memory_bytes);
    let cmd_display = if process.command.is_empty() {
        "(none)".to_string()
    } else {
        truncate_unicode(&process.command, 60)
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
            process.user_id.as_deref().unwrap_or("N/A").to_string(),
            theme,
        ),
        detail_line(
            "Group",
            process.group_id.as_deref().unwrap_or("N/A").to_string(),
            theme,
        ),
        detail_line(
            "GroupName",
            process.group_name.as_deref().unwrap_or("N/A").to_string(),
            theme,
        ),
        detail_line(
            "Priority",
            process
                .priority
                .map(|p| p.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            theme,
        ),
        detail_line(
            "I/O",
            process
                .io_stats
                .map(|io| {
                    format!(
                        "R {} / W {}",
                        format_bytes(io.read_bytes),
                        format_bytes(io.write_bytes)
                    )
                })
                .unwrap_or_else(|| "N/A".to_string()),
            theme,
        ),
        detail_line("Status", process.status.to_string(), theme),
        detail_line("Children", process.children.len().to_string(), theme),
    ];

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, chunks[0]);

    if let Some(hist) = history
        && hist.memory.len() > 1
        && chunks[1].height >= 4
    {
        let spark_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
            .split(chunks[1]);

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
            .max(10000)
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
