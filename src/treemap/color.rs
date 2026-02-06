use std::collections::HashMap;

use ratatui::style::Color;

use crate::system::process::ProcessTree;

use super::node::TreemapRect;

// ── Theme ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub header_accent_bg: Color,
    pub header_accent_fg: Color,
    pub header_stat_colors: [Color; 4], // cpu, ram, swap, procs
    pub mode_label: Color,
    pub selection_border: Color,
    pub key_hint: Color,
    pub status_ok: Color,
    pub status_err: Color,
    pub statusbar_bg: Color,
    pub detail_border: Color,
    pub detail_label: Color,
    pub heat_colors: [Color; 5], // cold → hot
    pub hash_palette: [Color; 8],
    pub mono_base: u8,
    pub mono_range: u8,
    // Extended palette for modern TUI redesign
    pub base_bg: Color,
    pub surface_bg: Color,
    pub overlay_border: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub accent_mauve: Color,
    pub pill_key_bg: Color,
    pub pill_key_fg: Color,
    pub pill_desc_fg: Color,
    pub gauge_filled: Color,
    pub gauge_unfilled: Color,
    pub sparkline_color: Color,
}

impl Theme {
    pub fn from_config(theme_name: &str) -> Self {
        match theme_name.to_lowercase().as_str() {
            "light" => Self::light(),
            "colorblind" => Self::colorblind(),
            "vivid" => Self::vivid(),
            _ => Self::dark(),
        }
    }

    pub fn next(&self) -> Self {
        match self.name {
            "dark" => Self::vivid(),
            "vivid" => Self::light(),
            "light" => Self::colorblind(),
            _ => Self::dark(),
        }
    }

    pub fn dark() -> Self {
        Theme {
            name: "dark",
            header_accent_bg: Color::Green,
            header_accent_fg: Color::Black,
            header_stat_colors: [Color::Cyan, Color::Yellow, Color::Magenta, Color::White],
            mode_label: Color::LightGreen,
            selection_border: Color::Yellow,
            key_hint: Color::Yellow,
            status_ok: Color::Green,
            status_err: Color::Red,
            statusbar_bg: Color::DarkGray,
            detail_border: Color::DarkGray,
            detail_label: Color::Yellow,
            heat_colors: [
                Color::DarkGray,
                Color::Green,
                Color::Yellow,
                Color::LightRed,
                Color::Red,
            ],
            hash_palette: [
                Color::Blue,
                Color::Green,
                Color::Cyan,
                Color::Magenta,
                Color::Yellow,
                Color::LightBlue,
                Color::LightGreen,
                Color::LightCyan,
            ],
            mono_base: 40,
            mono_range: 180,
            base_bg: Color::Rgb(30, 30, 30),
            surface_bg: Color::DarkGray,
            overlay_border: Color::DarkGray,
            text_primary: Color::White,
            text_secondary: Color::Gray,
            accent_mauve: Color::Green,
            pill_key_bg: Color::Yellow,
            pill_key_fg: Color::Black,
            pill_desc_fg: Color::White,
            gauge_filled: Color::Yellow,
            gauge_unfilled: Color::DarkGray,
            sparkline_color: Color::Cyan,
        }
    }

    pub fn light() -> Self {
        Theme {
            name: "light",
            header_accent_bg: Color::Blue,
            header_accent_fg: Color::White,
            header_stat_colors: [
                Color::DarkGray,
                Color::Rgb(180, 100, 0),
                Color::Magenta,
                Color::DarkGray,
            ],
            mode_label: Color::Blue,
            selection_border: Color::Rgb(200, 100, 0),
            key_hint: Color::Blue,
            status_ok: Color::Rgb(0, 120, 0),
            status_err: Color::Red,
            statusbar_bg: Color::Rgb(200, 200, 200),
            detail_border: Color::Rgb(150, 150, 150),
            detail_label: Color::Blue,
            heat_colors: [
                Color::Rgb(180, 180, 180),
                Color::Rgb(100, 180, 100),
                Color::Rgb(220, 180, 50),
                Color::Rgb(220, 120, 80),
                Color::Rgb(200, 60, 60),
            ],
            hash_palette: [
                Color::Rgb(70, 130, 180),
                Color::Rgb(60, 160, 60),
                Color::Rgb(0, 150, 150),
                Color::Rgb(160, 80, 160),
                Color::Rgb(200, 170, 50),
                Color::Rgb(100, 160, 210),
                Color::Rgb(100, 190, 100),
                Color::Rgb(80, 180, 180),
            ],
            mono_base: 100,
            mono_range: 120,
            base_bg: Color::Rgb(240, 240, 240),
            surface_bg: Color::Rgb(200, 200, 200),
            overlay_border: Color::Rgb(150, 150, 150),
            text_primary: Color::Black,
            text_secondary: Color::DarkGray,
            accent_mauve: Color::Blue,
            pill_key_bg: Color::Blue,
            pill_key_fg: Color::White,
            pill_desc_fg: Color::Black,
            gauge_filled: Color::Rgb(70, 130, 180),
            gauge_unfilled: Color::Rgb(200, 200, 200),
            sparkline_color: Color::Rgb(70, 130, 180),
        }
    }

    pub fn colorblind() -> Self {
        // Blue-orange diverging palette, safe for deuteranopia/protanopia
        Theme {
            name: "colorblind",
            header_accent_bg: Color::Rgb(0, 114, 178),
            header_accent_fg: Color::White,
            header_stat_colors: [
                Color::Rgb(86, 180, 233),
                Color::Rgb(230, 159, 0),
                Color::Rgb(204, 121, 167),
                Color::White,
            ],
            mode_label: Color::Rgb(86, 180, 233),
            selection_border: Color::Rgb(240, 228, 66),
            key_hint: Color::Rgb(230, 159, 0),
            status_ok: Color::Rgb(0, 158, 115),
            status_err: Color::Rgb(213, 94, 0),
            statusbar_bg: Color::DarkGray,
            detail_border: Color::Rgb(86, 180, 233),
            detail_label: Color::Rgb(230, 159, 0),
            heat_colors: [
                Color::Rgb(80, 80, 80),
                Color::Rgb(0, 114, 178),
                Color::Rgb(86, 180, 233),
                Color::Rgb(230, 159, 0),
                Color::Rgb(213, 94, 0),
            ],
            hash_palette: [
                Color::Rgb(0, 114, 178),
                Color::Rgb(230, 159, 0),
                Color::Rgb(0, 158, 115),
                Color::Rgb(204, 121, 167),
                Color::Rgb(86, 180, 233),
                Color::Rgb(240, 228, 66),
                Color::Rgb(213, 94, 0),
                Color::Rgb(128, 128, 128),
            ],
            mono_base: 40,
            mono_range: 180,
            base_bg: Color::Rgb(30, 30, 30),
            surface_bg: Color::DarkGray,
            overlay_border: Color::Rgb(86, 180, 233),
            text_primary: Color::White,
            text_secondary: Color::Gray,
            accent_mauve: Color::Rgb(86, 180, 233),
            pill_key_bg: Color::Rgb(230, 159, 0),
            pill_key_fg: Color::Black,
            pill_desc_fg: Color::White,
            gauge_filled: Color::Rgb(0, 158, 115),
            gauge_unfilled: Color::DarkGray,
            sparkline_color: Color::Rgb(86, 180, 233),
        }
    }

    pub fn vivid() -> Self {
        Theme {
            name: "vivid",
            header_accent_bg: Color::Rgb(203, 166, 247),
            header_accent_fg: Color::Rgb(30, 30, 46),
            header_stat_colors: [
                Color::Rgb(137, 180, 250), // blue (cpu)
                Color::Rgb(166, 227, 161), // green (ram)
                Color::Rgb(249, 226, 175), // yellow (swap)
                Color::Rgb(205, 214, 244), // text (procs)
            ],
            mode_label: Color::Rgb(203, 166, 247),
            selection_border: Color::Rgb(137, 180, 250),
            key_hint: Color::Rgb(203, 166, 247),
            status_ok: Color::Rgb(166, 227, 161),
            status_err: Color::Rgb(243, 139, 168),
            statusbar_bg: Color::Rgb(49, 50, 68),
            detail_border: Color::Rgb(69, 71, 90),
            detail_label: Color::Rgb(203, 166, 247),
            heat_colors: [
                Color::Rgb(69, 71, 90),
                Color::Rgb(166, 227, 161),
                Color::Rgb(249, 226, 175),
                Color::Rgb(250, 179, 135),
                Color::Rgb(243, 139, 168),
            ],
            hash_palette: [
                Color::Rgb(137, 180, 250),
                Color::Rgb(166, 227, 161),
                Color::Rgb(148, 226, 213),
                Color::Rgb(203, 166, 247),
                Color::Rgb(249, 226, 175),
                Color::Rgb(116, 199, 236),
                Color::Rgb(245, 194, 231),
                Color::Rgb(250, 179, 135),
            ],
            mono_base: 30,
            mono_range: 170,
            base_bg: Color::Rgb(30, 30, 46),
            surface_bg: Color::Rgb(49, 50, 68),
            overlay_border: Color::Rgb(69, 71, 90),
            text_primary: Color::Rgb(205, 214, 244),
            text_secondary: Color::Rgb(166, 173, 200),
            accent_mauve: Color::Rgb(203, 166, 247),
            pill_key_bg: Color::Rgb(203, 166, 247),
            pill_key_fg: Color::Rgb(30, 30, 46),
            pill_desc_fg: Color::Rgb(205, 214, 244),
            gauge_filled: Color::Rgb(166, 227, 161),
            gauge_unfilled: Color::Rgb(69, 71, 90),
            sparkline_color: Color::Rgb(137, 180, 250),
        }
    }
}

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
    theme: &Theme,
) {
    match mode {
        ColorMode::ByMemory => apply_memory_heatmap(rects, total_memory, theme),
        ColorMode::ByCpu => apply_cpu_heatmap(rects, process_tree, theme),
        ColorMode::ByUser => apply_user_colors(rects, process_tree, theme),
        ColorMode::ByGroup => apply_group_colors(rects, process_tree, theme),
        ColorMode::Monochrome => apply_monochrome(rects, total_memory, theme),
    }
}

pub fn apply_memory_heatmap(rects: &mut [TreemapRect], total_memory: u64, theme: &Theme) {
    for rect in rects.iter_mut() {
        rect.color = memory_color(rect.value, total_memory, theme);
    }
}

fn memory_color(memory_bytes: u64, total_memory: u64, theme: &Theme) -> Color {
    if total_memory == 0 {
        return theme.heat_colors[0];
    }
    let pct = memory_bytes as f64 / total_memory as f64;
    if pct > 0.15 {
        theme.heat_colors[4]
    } else if pct > 0.08 {
        theme.heat_colors[3]
    } else if pct > 0.04 {
        theme.heat_colors[2]
    } else if pct > 0.02 {
        theme.heat_colors[1]
    } else {
        theme.heat_colors[0]
    }
}

fn apply_cpu_heatmap(rects: &mut [TreemapRect], process_tree: &ProcessTree, theme: &Theme) {
    for rect in rects.iter_mut() {
        let cpu = process_tree
            .processes
            .get(&rect.id)
            .map(|p| p.cpu_percent)
            .unwrap_or(0.0);
        rect.color = cpu_color(cpu, theme);
    }
}

fn cpu_color(cpu_percent: f32, theme: &Theme) -> Color {
    if cpu_percent > 80.0 {
        theme.heat_colors[4]
    } else if cpu_percent > 40.0 {
        theme.heat_colors[3]
    } else if cpu_percent > 20.0 {
        theme.heat_colors[2]
    } else if cpu_percent > 5.0 {
        theme.heat_colors[1]
    } else {
        theme.heat_colors[0]
    }
}

fn apply_user_colors(rects: &mut [TreemapRect], process_tree: &ProcessTree, theme: &Theme) {
    apply_hash_colors(rects, process_tree, theme, |p| {
        p.user_id.clone().unwrap_or_default()
    });
}

fn apply_group_colors(rects: &mut [TreemapRect], process_tree: &ProcessTree, theme: &Theme) {
    apply_hash_colors(rects, process_tree, theme, |p| {
        p.group_id.clone().unwrap_or_default()
    });
}

fn apply_hash_colors(
    rects: &mut [TreemapRect],
    process_tree: &ProcessTree,
    theme: &Theme,
    key_fn: impl Fn(&crate::system::process::ProcessInfo) -> String,
) {
    let mut color_map: HashMap<String, Color> = HashMap::new();
    let mut next_idx = 0;

    for rect in rects.iter_mut() {
        let key = process_tree
            .processes
            .get(&rect.id)
            .map(&key_fn)
            .unwrap_or_default();

        let color = *color_map.entry(key).or_insert_with(|| {
            let c = theme.hash_palette[next_idx % theme.hash_palette.len()];
            next_idx += 1;
            c
        });
        rect.color = color;
    }
}

fn apply_monochrome(rects: &mut [TreemapRect], total_memory: u64, theme: &Theme) {
    for rect in rects.iter_mut() {
        if total_memory == 0 {
            rect.color = Color::Rgb(80, 80, 80);
            continue;
        }
        let frac = (rect.value as f64 / total_memory as f64).clamp(0.0, 1.0);
        let gray = (theme.mono_base as f64 + frac * theme.mono_range as f64) as u8;
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
        let theme = Theme::dark();
        let mut rects = vec![
            make_rect(1, 200_000_000), // ~19% of total — should be hot
            make_rect(2, 50_000_000),  // ~4.8% — should be mid
            make_rect(3, 10_000_000),  // ~1% — should be cold
        ];
        let total = 1_024_000_000;
        apply_memory_heatmap(&mut rects, total, &theme);
        assert_eq!(rects[0].color, theme.heat_colors[4]);
        assert_eq!(rects[1].color, theme.heat_colors[2]);
        assert_eq!(rects[2].color, theme.heat_colors[0]);
    }

    #[test]
    fn cpu_heatmap_assigns_colors() {
        let theme = Theme::dark();
        let procs = vec![
            make_process(1, 100, 90.0),
            make_process(2, 100, 3.0),
        ];
        let tree = make_tree(procs);
        let mut rects = vec![make_rect(1, 100), make_rect(2, 100)];
        apply_color_mode(&mut rects, ColorMode::ByCpu, &tree, 200, &theme);
        assert_eq!(rects[0].color, theme.heat_colors[4]);
        assert_eq!(rects[1].color, theme.heat_colors[0]);
    }

    #[test]
    fn monochrome_produces_rgb() {
        let theme = Theme::dark();
        let mut rects = vec![make_rect(1, 500), make_rect(2, 100)];
        apply_monochrome(&mut rects, 1000, &theme);
        for r in &rects {
            match r.color {
                Color::Rgb(_, _, _) => {}
                _ => panic!("Expected Rgb color, got {:?}", r.color),
            }
        }
    }

    #[test]
    fn user_colors_same_user_same_color() {
        let theme = Theme::dark();
        let procs = vec![
            make_process(1, 100, 10.0), // user_0 (1%3=1 -> user_1)
            make_process(4, 100, 10.0), // user_1 (4%3=1 -> user_1)
        ];
        let tree = make_tree(procs);
        let mut rects = vec![make_rect(1, 100), make_rect(4, 100)];
        apply_color_mode(&mut rects, ColorMode::ByUser, &tree, 200, &theme);
        assert_eq!(rects[0].color, rects[1].color);
    }

    #[test]
    fn theme_from_config_presets() {
        let dark = Theme::from_config("dark");
        assert_eq!(dark.name, "dark");
        let vivid = Theme::from_config("vivid");
        assert_eq!(vivid.name, "vivid");
        let light = Theme::from_config("light");
        assert_eq!(light.name, "light");
        let cb = Theme::from_config("colorblind");
        assert_eq!(cb.name, "colorblind");
    }

    #[test]
    fn theme_cycles() {
        let t = Theme::dark();
        let t = t.next();
        assert_eq!(t.name, "vivid");
        let t = t.next();
        assert_eq!(t.name, "light");
        let t = t.next();
        assert_eq!(t.name, "colorblind");
        let t = t.next();
        assert_eq!(t.name, "dark");
    }
}
