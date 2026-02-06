use std::collections::VecDeque;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, Paragraph, Sparkline};
use ratatui::Frame;

use crate::system::snapshot::SystemSnapshot;
use crate::treemap::color::{ColorMode, Theme};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    snapshot: &SystemSnapshot,
    color_mode: ColorMode,
    theme: &Theme,
    breadcrumbs: &[(u32, String)],
    cpu_history: &VecDeque<u64>,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(area);

    // Block 1: Branding + breadcrumbs + mode + theme
    render_branding(frame, chunks[0], snapshot, color_mode, theme, breadcrumbs);

    // Block 2: RAM Gauge
    render_ram_gauge(frame, chunks[1], snapshot, theme);

    // Block 3: CPU Sparkline
    render_cpu_sparkline(frame, chunks[2], snapshot, theme, cpu_history);
}

fn render_branding(
    frame: &mut Frame,
    area: Rect,
    snapshot: &SystemSnapshot,
    color_mode: ColorMode,
    theme: &Theme,
    breadcrumbs: &[(u32, String)],
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.overlay_border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut spans = vec![
        Span::styled(
            " treetop ",
            Style::default()
                .fg(theme.header_accent_fg)
                .bg(theme.header_accent_bg)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    for (_, name) in breadcrumbs {
        spans.push(Span::styled(
            " > ",
            Style::default().fg(theme.text_secondary),
        ));
        spans.push(Span::styled(
            name.as_str(),
            Style::default()
                .fg(theme.accent_mauve)
                .add_modifier(Modifier::BOLD),
        ));
    }

    spans.extend([
        Span::raw("  "),
        Span::styled(
            color_mode.label().to_string(),
            Style::default().fg(theme.text_secondary),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Procs: {}", snapshot.process_tree.processes.len()),
            Style::default().fg(theme.text_secondary),
        ),
    ]);

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), inner);
}

fn render_ram_gauge(
    frame: &mut Frame,
    area: Rect,
    snapshot: &SystemSnapshot,
    theme: &Theme,
) {
    let ram_used_mb = snapshot.memory_used / 1_048_576;
    let ram_total_mb = snapshot.memory_total / 1_048_576;
    let ram_ratio = if snapshot.memory_total > 0 {
        (snapshot.memory_used as f64 / snapshot.memory_total as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let ram_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.overlay_border))
        .title(Span::styled(
            " RAM ",
            Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD),
        ));

    let gauge = Gauge::default()
        .block(ram_block)
        .gauge_style(
            Style::default()
                .fg(theme.gauge_filled)
                .bg(theme.gauge_unfilled),
        )
        .ratio(ram_ratio)
        .label(format!(
            "{}/{} MB ({:.0}%)",
            ram_used_mb,
            ram_total_mb,
            ram_ratio * 100.0
        ));

    frame.render_widget(gauge, area);
}

fn render_cpu_sparkline(
    frame: &mut Frame,
    area: Rect,
    snapshot: &SystemSnapshot,
    theme: &Theme,
    cpu_history: &VecDeque<u64>,
) {
    let cpu_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.overlay_border))
        .title(Span::styled(
            format!(" CPU {:.0}% ", snapshot.cpu_usage_percent),
            Style::default()
                .fg(theme.text_secondary)
                .add_modifier(Modifier::BOLD),
        ));

    let cpu_data: Vec<u64> = cpu_history.iter().copied().collect();
    let sparkline = Sparkline::default()
        .block(cpu_block)
        .data(&cpu_data)
        .max(10000)
        .style(Style::default().fg(theme.sparkline_color));

    frame.render_widget(sparkline, area);
}
