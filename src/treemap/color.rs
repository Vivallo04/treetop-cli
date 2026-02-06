use std::collections::HashMap;

use ratatui::style::Color;

use crate::system::process::ProcessTree;

use super::node::TreemapRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    ByMemory,
    ByCpu,
    ByUser,
    ByGroup,
    Monochrome,
}

impl ColorMode {
    pub fn next(self) -> Self {
        match self {
            ColorMode::ByMemory => ColorMode::ByCpu,
            ColorMode::ByCpu => ColorMode::ByUser,
            ColorMode::ByUser => ColorMode::ByGroup,
            ColorMode::ByGroup => ColorMode::Monochrome,
            ColorMode::Monochrome => ColorMode::ByMemory,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ColorMode::ByMemory => "Memory",
            ColorMode::ByCpu => "CPU",
            ColorMode::ByUser => "User",
            ColorMode::ByGroup => "Group",
            ColorMode::Monochrome => "Mono",
        }
    }

    pub fn from_str_config(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cpu" => ColorMode::ByCpu,
            "user" => ColorMode::ByUser,
            "group" => ColorMode::ByGroup,
            "mono" | "monochrome" => ColorMode::Monochrome,
            _ => ColorMode::ByMemory,
        }
    }
}

pub fn apply_color_mode(
    rects: &mut [TreemapRect],
    mode: ColorMode,
    process_tree: &ProcessTree,
    total_memory: u64,
) {
    match mode {
        ColorMode::ByMemory => apply_memory_heatmap(rects, total_memory),
        ColorMode::ByCpu => apply_cpu_heatmap(rects, process_tree),
        ColorMode::ByUser => apply_user_colors(rects, process_tree),
        ColorMode::ByGroup => apply_group_colors(rects, process_tree),
        ColorMode::Monochrome => apply_monochrome(rects, total_memory),
    }
}

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

fn apply_cpu_heatmap(rects: &mut [TreemapRect], process_tree: &ProcessTree) {
    for rect in rects.iter_mut() {
        let cpu = process_tree
            .processes
            .get(&rect.id)
            .map(|p| p.cpu_percent)
            .unwrap_or(0.0);
        rect.color = cpu_color(cpu);
    }
}

fn cpu_color(cpu_percent: f32) -> Color {
    if cpu_percent > 80.0 {
        Color::Red
    } else if cpu_percent > 40.0 {
        Color::LightRed
    } else if cpu_percent > 20.0 {
        Color::Yellow
    } else if cpu_percent > 5.0 {
        Color::Green
    } else {
        Color::DarkGray
    }
}

fn apply_user_colors(rects: &mut [TreemapRect], process_tree: &ProcessTree) {
    apply_hash_colors(rects, process_tree, |p| {
        p.user_id.clone().unwrap_or_default()
    });
}

fn apply_group_colors(rects: &mut [TreemapRect], process_tree: &ProcessTree) {
    apply_hash_colors(rects, process_tree, |p| {
        p.group_id.clone().unwrap_or_default()
    });
}

fn apply_hash_colors(
    rects: &mut [TreemapRect],
    process_tree: &ProcessTree,
    key_fn: impl Fn(&crate::system::process::ProcessInfo) -> String,
) {
    const PALETTE: [Color; 8] = [
        Color::Blue,
        Color::Green,
        Color::Cyan,
        Color::Magenta,
        Color::Yellow,
        Color::LightBlue,
        Color::LightGreen,
        Color::LightCyan,
    ];

    let mut color_map: HashMap<String, Color> = HashMap::new();
    let mut next_idx = 0;

    for rect in rects.iter_mut() {
        let key = process_tree
            .processes
            .get(&rect.id)
            .map(&key_fn)
            .unwrap_or_default();

        let color = *color_map.entry(key).or_insert_with(|| {
            let c = PALETTE[next_idx % PALETTE.len()];
            next_idx += 1;
            c
        });
        rect.color = color;
    }
}

fn apply_monochrome(rects: &mut [TreemapRect], total_memory: u64) {
    for rect in rects.iter_mut() {
        if total_memory == 0 {
            rect.color = Color::Rgb(80, 80, 80);
            continue;
        }
        let frac = (rect.value as f64 / total_memory as f64).clamp(0.0, 1.0);
        let gray = (40.0 + frac * 180.0) as u8;
        rect.color = Color::Rgb(gray, gray, gray);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::process::{ProcessInfo, ProcessTree};
    use crate::treemap::node::LayoutRect;

    fn make_rect(id: u32, value: u64) -> TreemapRect {
        TreemapRect {
            rect: LayoutRect::new(0.0, 0.0, 10.0, 10.0),
            id,
            label: format!("proc_{id}"),
            value,
            color: Color::Reset,
        }
    }

    fn make_process(pid: u32, memory: u64, cpu: f32) -> ProcessInfo {
        ProcessInfo {
            pid,
            ppid: 1,
            name: format!("proc_{pid}"),
            command: String::new(),
            memory_bytes: memory,
            cpu_percent: cpu,
            user_id: Some(format!("user_{}", pid % 3)),
            group_id: Some(format!("group_{}", pid % 2)),
            status: "Running".to_string(),
            children: Vec::new(),
        }
    }

    fn make_tree(procs: Vec<ProcessInfo>) -> ProcessTree {
        let total_memory: u64 = procs.iter().map(|p| p.memory_bytes).sum();
        let mut processes = std::collections::HashMap::new();
        for p in procs {
            processes.insert(p.pid, p);
        }
        ProcessTree {
            processes,
            roots: Vec::new(),
            total_memory,
        }
    }

    #[test]
    fn color_mode_cycles() {
        let mut mode = ColorMode::ByMemory;
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByCpu);
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByUser);
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByGroup);
        mode = mode.next();
        assert_eq!(mode, ColorMode::Monochrome);
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByMemory);
    }

    #[test]
    fn color_mode_labels() {
        assert_eq!(ColorMode::ByMemory.label(), "Memory");
        assert_eq!(ColorMode::ByCpu.label(), "CPU");
        assert_eq!(ColorMode::ByUser.label(), "User");
        assert_eq!(ColorMode::ByGroup.label(), "Group");
        assert_eq!(ColorMode::Monochrome.label(), "Mono");
    }

    #[test]
    fn color_mode_from_config() {
        assert_eq!(ColorMode::from_str_config("memory"), ColorMode::ByMemory);
        assert_eq!(ColorMode::from_str_config("cpu"), ColorMode::ByCpu);
        assert_eq!(ColorMode::from_str_config("user"), ColorMode::ByUser);
        assert_eq!(ColorMode::from_str_config("group"), ColorMode::ByGroup);
        assert_eq!(ColorMode::from_str_config("mono"), ColorMode::Monochrome);
        assert_eq!(
            ColorMode::from_str_config("unknown"),
            ColorMode::ByMemory
        );
    }

    #[test]
    fn memory_heatmap_assigns_colors() {
        let mut rects = vec![
            make_rect(1, 200_000_000), // ~19% of total — should be Red
            make_rect(2, 50_000_000),  // ~4.8% — should be Yellow
            make_rect(3, 10_000_000),  // ~1% — should be DarkGray
        ];
        let total = 1_024_000_000;
        apply_memory_heatmap(&mut rects, total);
        assert_eq!(rects[0].color, Color::Red);
        assert_eq!(rects[1].color, Color::Yellow);
        assert_eq!(rects[2].color, Color::DarkGray);
    }

    #[test]
    fn cpu_heatmap_assigns_colors() {
        let procs = vec![
            make_process(1, 100, 90.0),
            make_process(2, 100, 3.0),
        ];
        let tree = make_tree(procs);
        let mut rects = vec![make_rect(1, 100), make_rect(2, 100)];
        apply_color_mode(&mut rects, ColorMode::ByCpu, &tree, 200);
        assert_eq!(rects[0].color, Color::Red);
        assert_eq!(rects[1].color, Color::DarkGray);
    }

    #[test]
    fn monochrome_produces_rgb() {
        let mut rects = vec![make_rect(1, 500), make_rect(2, 100)];
        apply_monochrome(&mut rects, 1000);
        for r in &rects {
            match r.color {
                Color::Rgb(_, _, _) => {}
                _ => panic!("Expected Rgb color, got {:?}", r.color),
            }
        }
    }

    #[test]
    fn user_colors_same_user_same_color() {
        let procs = vec![
            make_process(1, 100, 10.0), // user_0 (1%3=1 -> user_1)
            make_process(4, 100, 10.0), // user_1 (4%3=1 -> user_1)
        ];
        let tree = make_tree(procs);
        let mut rects = vec![make_rect(1, 100), make_rect(4, 100)];
        apply_color_mode(&mut rects, ColorMode::ByUser, &tree, 200);
        assert_eq!(rects[0].color, rects[1].color);
    }
}
