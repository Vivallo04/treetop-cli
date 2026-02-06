use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::system::snapshot::SystemSnapshot;

pub fn render(frame: &mut Frame, area: Rect, snapshot: &SystemSnapshot) {
    let cpu_str = format!("CPU: {:.1}%", snapshot.cpu_usage_percent);

    let ram_used_mb = snapshot.memory_used / 1_048_576;
    let ram_total_mb = snapshot.memory_total / 1_048_576;
    let ram_pct = if snapshot.memory_total > 0 {
        (snapshot.memory_used as f64 / snapshot.memory_total as f64) * 100.0
    } else {
        0.0
    };
    let ram_str = format!("RAM: {}/{} MB ({:.1}%)", ram_used_mb, ram_total_mb, ram_pct);

    let swap_str = if snapshot.swap_total > 0 {
        let swap_used_mb = snapshot.swap_used / 1_048_576;
        let swap_total_mb = snapshot.swap_total / 1_048_576;
        format!("Swap: {}/{} MB", swap_used_mb, swap_total_mb)
    } else {
        "Swap: N/A".to_string()
    };

    let procs = format!("Procs: {}", snapshot.process_tree.processes.len());

    let header_line = Line::from(vec![
        Span::styled(
            " treetop ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(cpu_str, Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(ram_str, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(swap_str, Style::default().fg(Color::Magenta)),
        Span::raw("  "),
        Span::styled(procs, Style::default().fg(Color::White)),
    ]);

    frame.render_widget(Paragraph::new(header_line), area);
}
