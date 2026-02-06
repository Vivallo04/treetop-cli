use ratatui::style::Color;

use super::node::TreemapRect;

pub fn apply_memory_heatmap(rects: &mut [TreemapRect], total_memory: u64) {
    for rect in rects.iter_mut() {
        rect.color = memory_color(rect.value, total_memory);
    }
}

fn memory_color(memory_bytes: u64, total_memory: u64) -> Color {
    if total_memory == 0 {
        return Color::DarkGray;
    }
    let pct = memory_bytes as f64 / total_memory as f64;
    if pct > 0.15 {
        Color::Red
    } else if pct > 0.08 {
        Color::LightRed
    } else if pct > 0.04 {
        Color::Yellow
    } else if pct > 0.02 {
        Color::Green
    } else {
        Color::DarkGray
    }
}
