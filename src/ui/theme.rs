use ratatui::style::Color;
use ratatui::widgets::BorderType;
use std::hash::{Hash, Hasher};

use crate::config::ColorsConfig;
use crate::system::process::ProcessTree;
use crate::treemap::node::TreemapRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    ByName,
    ByMemory,
    ByCpu,
    ByUser,
    ByGroup,
    Monochrome,
}

impl ColorMode {
    pub fn next(self) -> Self {
        match self {
            ColorMode::ByName => ColorMode::ByMemory,
            ColorMode::ByMemory => ColorMode::ByCpu,
            ColorMode::ByCpu => ColorMode::ByUser,
            ColorMode::ByUser => ColorMode::ByGroup,
            ColorMode::ByGroup => ColorMode::Monochrome,
            ColorMode::Monochrome => ColorMode::ByName,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ColorMode::ByName => "Name",
            ColorMode::ByMemory => "Memory",
            ColorMode::ByCpu => "CPU",
            ColorMode::ByUser => "User",
            ColorMode::ByGroup => "Group",
            ColorMode::Monochrome => "Mono",
        }
    }

    pub fn from_str_config(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "name" | "process" => ColorMode::ByName,
            "cpu" => ColorMode::ByCpu,
            "user" => ColorMode::ByUser,
            "group" => ColorMode::ByGroup,
            "mono" | "monochrome" => ColorMode::Monochrome,
            _ => ColorMode::ByMemory,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSupport {
    Auto,
    Truecolor,
    Color256,
    Mono,
}

impl ColorSupport {
    pub fn from_config_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "truecolor" | "24bit" => ColorSupport::Truecolor,
            "256" | "256color" => ColorSupport::Color256,
            "mono" | "monochrome" => ColorSupport::Mono,
            _ => ColorSupport::Auto,
        }
    }
}

pub fn detect_color_support() -> ColorSupport {
    let colorterm = std::env::var("COLORTERM")
        .unwrap_or_default()
        .to_lowercase();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return ColorSupport::Truecolor;
    }

    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    if term.contains("256color") {
        return ColorSupport::Color256;
    }
    ColorSupport::Color256
}

pub fn resolve_color_support(config: &str) -> ColorSupport {
    let parsed = ColorSupport::from_config_str(config);
    if parsed == ColorSupport::Auto {
        detect_color_support()
    } else {
        parsed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    Rounded,
    Thin,
}

impl BorderStyle {
    pub fn from_config_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rounded" => BorderStyle::Rounded,
            _ => BorderStyle::Thin,
        }
    }

    pub fn border_type(self) -> BorderType {
        match self {
            BorderStyle::Rounded => BorderType::Rounded,
            BorderStyle::Thin => BorderType::Plain,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeatOverrides {
    pub low: String,
    pub mid: String,
    pub high: String,
}

impl HeatOverrides {
    pub fn from_config(colors: &ColorsConfig) -> Self {
        Self {
            low: colors.heat_low.clone(),
            mid: colors.heat_mid.clone(),
            high: colors.heat_high.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub header_accent_bg: Color,
    pub header_accent_fg: Color,
    pub selection_border: Color,
    pub status_ok: Color,
    pub status_err: Color,
    pub statusbar_bg: Color,
    pub overlay_border: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub accent_mauve: Color,
    pub pill_key_bg: Color,
    pub pill_key_fg: Color,
    pub pill_desc_fg: Color,
    pub surface_bg: Color,
    pub gauge_filled: Color,
    pub gauge_unfilled: Color,
    pub sparkline_color: Color,
    pub other_group_bg: Color,
    pub heat_colors: [Color; 5],
    pub hash_palette: [Color; 8],
    pub mono_base: u8,
    pub mono_range: u8,
}

impl Theme {
    pub fn from_config(theme_name: &str, heat: &HeatOverrides, support: ColorSupport) -> Self {
        let mut theme = match theme_name.to_lowercase().as_str() {
            "light" => Self::light(),
            "colorblind" => Self::colorblind(),
            "vivid" => Self::vivid(),
            _ => Self::dark(),
        };

        if support == ColorSupport::Mono {
            theme = Self::mono();
        }

        theme.apply_heat_overrides(heat);
        theme.apply_color_support(support);
        theme
    }

    pub fn next(&self, heat: &HeatOverrides, support: ColorSupport) -> Self {
        if support == ColorSupport::Mono {
            return Self::mono();
        }
        let next_name = match self.name {
            "dark" => "vivid",
            "vivid" => "light",
            "light" => "colorblind",
            _ => "dark",
        };
        Theme::from_config(next_name, heat, support)
    }

    fn apply_heat_overrides(&mut self, heat: &HeatOverrides) {
        let low = parse_hex_color(&heat.low);
        let mid = parse_hex_color(&heat.mid);
        let high = parse_hex_color(&heat.high);

        if let (Some(low), Some(mid), Some(high)) = (low, mid, high) {
            // Keep semantic healthy/danger colors stable while allowing config anchors
            // for idle (low), warning (mid), and critical (high).
            self.heat_colors = [low, self.heat_colors[1], mid, self.heat_colors[3], high];
        }
    }

    fn apply_color_support(&mut self, support: ColorSupport) {
        let map = |c: Color| adapt_color(c, support);

        self.header_accent_bg = map(self.header_accent_bg);
        self.header_accent_fg = map(self.header_accent_fg);
        self.selection_border = map(self.selection_border);
        self.status_ok = map(self.status_ok);
        self.status_err = map(self.status_err);
        self.statusbar_bg = map(self.statusbar_bg);
        self.overlay_border = map(self.overlay_border);
        self.text_primary = map(self.text_primary);
        self.text_secondary = map(self.text_secondary);
        self.accent_mauve = map(self.accent_mauve);
        self.pill_key_bg = map(self.pill_key_bg);
        self.pill_key_fg = map(self.pill_key_fg);
        self.pill_desc_fg = map(self.pill_desc_fg);
        self.surface_bg = map(self.surface_bg);
        self.gauge_filled = map(self.gauge_filled);
        self.gauge_unfilled = map(self.gauge_unfilled);
        self.sparkline_color = map(self.sparkline_color);
        self.other_group_bg = map(self.other_group_bg);

        self.heat_colors = self.heat_colors.map(map);
        self.hash_palette = self.hash_palette.map(map);
    }

    pub fn dark() -> Self {
        Theme {
            name: "dark",
            header_accent_bg: Color::Green,
            header_accent_fg: Color::Black,
            selection_border: Color::White,
            status_ok: Color::Green,
            status_err: Color::Red,
            statusbar_bg: Color::DarkGray,
            overlay_border: Color::DarkGray,
            text_primary: Color::White,
            text_secondary: Color::Gray,
            accent_mauve: Color::Green,
            pill_key_bg: Color::Yellow,
            pill_key_fg: Color::Black,
            pill_desc_fg: Color::White,
            surface_bg: Color::DarkGray,
            gauge_filled: Color::Rgb(103, 232, 249),
            gauge_unfilled: Color::DarkGray,
            sparkline_color: Color::Rgb(251, 146, 60),
            other_group_bg: Color::Rgb(35, 40, 51),
            heat_colors: [
                Color::Rgb(71, 85, 105),
                Color::Rgb(16, 185, 129),
                Color::Rgb(249, 115, 22),
                Color::Rgb(239, 68, 68),
                Color::Rgb(236, 72, 153),
            ],
            hash_palette: [
                Color::Rgb(192, 132, 252),
                Color::Rgb(96, 165, 250),
                Color::Rgb(34, 211, 238),
                Color::Rgb(45, 212, 191),
                Color::Rgb(52, 211, 153),
                Color::Rgb(251, 146, 60),
                Color::Rgb(248, 113, 113),
                Color::Rgb(129, 140, 248),
            ],
            mono_base: 40,
            mono_range: 180,
        }
    }

    pub fn light() -> Self {
        Theme {
            name: "light",
            header_accent_bg: Color::Blue,
            header_accent_fg: Color::White,
            selection_border: Color::Rgb(200, 100, 0),
            status_ok: Color::Rgb(0, 120, 0),
            status_err: Color::Red,
            statusbar_bg: Color::Rgb(220, 220, 220),
            overlay_border: Color::Rgb(150, 150, 150),
            text_primary: Color::Black,
            text_secondary: Color::DarkGray,
            accent_mauve: Color::Blue,
            pill_key_bg: Color::Blue,
            pill_key_fg: Color::White,
            pill_desc_fg: Color::Black,
            surface_bg: Color::Rgb(200, 200, 200),
            gauge_filled: Color::Rgb(70, 130, 180),
            gauge_unfilled: Color::Rgb(200, 200, 200),
            sparkline_color: Color::Rgb(70, 130, 180),
            other_group_bg: Color::Rgb(192, 196, 204),
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
        }
    }

    pub fn colorblind() -> Self {
        Theme {
            name: "colorblind",
            header_accent_bg: Color::Rgb(0, 114, 178),
            header_accent_fg: Color::White,
            selection_border: Color::Rgb(240, 228, 66),
            status_ok: Color::Rgb(0, 158, 115),
            status_err: Color::Rgb(213, 94, 0),
            statusbar_bg: Color::DarkGray,
            overlay_border: Color::Rgb(86, 180, 233),
            text_primary: Color::White,
            text_secondary: Color::Gray,
            accent_mauve: Color::Rgb(86, 180, 233),
            pill_key_bg: Color::Rgb(230, 159, 0),
            pill_key_fg: Color::Black,
            pill_desc_fg: Color::White,
            surface_bg: Color::DarkGray,
            gauge_filled: Color::Rgb(0, 158, 115),
            gauge_unfilled: Color::DarkGray,
            sparkline_color: Color::Rgb(86, 180, 233),
            other_group_bg: Color::Rgb(70, 70, 70),
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
        }
    }

    pub fn vivid() -> Self {
        Theme {
            name: "vivid",
            header_accent_bg: Color::Rgb(203, 166, 247),
            header_accent_fg: Color::Rgb(30, 30, 46),
            selection_border: Color::White,
            status_ok: Color::Rgb(166, 227, 161),
            status_err: Color::Rgb(243, 139, 168),
            statusbar_bg: Color::Rgb(49, 50, 68),
            overlay_border: Color::Rgb(69, 71, 90),
            text_primary: Color::Rgb(205, 214, 244),
            text_secondary: Color::Rgb(166, 173, 200),
            accent_mauve: Color::Rgb(203, 166, 247),
            pill_key_bg: Color::Rgb(203, 166, 247),
            pill_key_fg: Color::Rgb(30, 30, 46),
            pill_desc_fg: Color::Rgb(205, 214, 244),
            surface_bg: Color::Rgb(49, 50, 68),
            gauge_filled: Color::Rgb(125, 211, 252),
            gauge_unfilled: Color::Rgb(69, 71, 90),
            sparkline_color: Color::Rgb(251, 146, 60),
            other_group_bg: Color::Rgb(49, 50, 68),
            heat_colors: [
                Color::Rgb(71, 85, 105),
                Color::Rgb(16, 185, 129),
                Color::Rgb(249, 115, 22),
                Color::Rgb(239, 68, 68),
                Color::Rgb(236, 72, 153),
            ],
            hash_palette: [
                Color::Rgb(192, 132, 252),
                Color::Rgb(96, 165, 250),
                Color::Rgb(34, 211, 238),
                Color::Rgb(45, 212, 191),
                Color::Rgb(52, 211, 153),
                Color::Rgb(251, 146, 60),
                Color::Rgb(248, 113, 113),
                Color::Rgb(129, 140, 248),
            ],
            mono_base: 30,
            mono_range: 170,
        }
    }

    pub fn mono() -> Self {
        Theme {
            name: "mono",
            header_accent_bg: Color::Black,
            header_accent_fg: Color::White,
            selection_border: Color::White,
            status_ok: Color::White,
            status_err: Color::White,
            statusbar_bg: Color::Black,
            overlay_border: Color::White,
            text_primary: Color::White,
            text_secondary: Color::Gray,
            accent_mauve: Color::White,
            pill_key_bg: Color::White,
            pill_key_fg: Color::Black,
            pill_desc_fg: Color::White,
            surface_bg: Color::Black,
            gauge_filled: Color::White,
            gauge_unfilled: Color::Black,
            sparkline_color: Color::White,
            other_group_bg: Color::DarkGray,
            heat_colors: [
                Color::Black,
                Color::DarkGray,
                Color::Gray,
                Color::White,
                Color::White,
            ],
            hash_palette: [
                Color::Black,
                Color::DarkGray,
                Color::Gray,
                Color::White,
                Color::Black,
                Color::DarkGray,
                Color::Gray,
                Color::White,
            ],
            mono_base: 40,
            mono_range: 180,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ColoredTreemapRect {
    pub rect: crate::treemap::node::LayoutRect,
    pub id: u32,
    pub label: String,
    pub value: u64,
    pub color: Color,
}

impl ColoredTreemapRect {
    fn from_base(base: &TreemapRect, color: Color) -> Self {
        Self {
            rect: base.rect.clone(),
            id: base.id,
            label: base.label.clone(),
            value: base.value,
            color,
        }
    }
}

pub fn colorize_rects(
    rects: &[TreemapRect],
    process_tree: &ProcessTree,
    total_memory: u64,
    mode: ColorMode,
    theme: &Theme,
    support: ColorSupport,
) -> Vec<ColoredTreemapRect> {
    let mode = if support == ColorSupport::Mono {
        ColorMode::Monochrome
    } else {
        mode
    };

    let mut colored: Vec<ColoredTreemapRect> = rects
        .iter()
        .map(|r| ColoredTreemapRect::from_base(r, Color::Reset))
        .collect();

    match mode {
        ColorMode::ByName => apply_name_colors(&mut colored, process_tree, theme),
        ColorMode::ByMemory => apply_memory_heatmap(&mut colored, total_memory, theme),
        ColorMode::ByCpu => apply_cpu_heatmap(&mut colored, process_tree, theme),
        ColorMode::ByUser => apply_user_colors(&mut colored, process_tree, theme),
        ColorMode::ByGroup => apply_group_colors(&mut colored, process_tree, theme),
        ColorMode::Monochrome => apply_monochrome(&mut colored, total_memory, theme),
    }

    for rect in &mut colored {
        if rect.id == 0 {
            rect.color = theme.other_group_bg;
        }
    }

    for rect in colored.iter_mut() {
        rect.color = adapt_color(rect.color, support);
    }

    colored
}

fn apply_name_colors(rects: &mut [ColoredTreemapRect], process_tree: &ProcessTree, theme: &Theme) {
    for rect in rects.iter_mut() {
        let process_name = process_tree
            .processes
            .get(&rect.id)
            .map(|p| p.name.as_str())
            .unwrap_or(rect.label.as_str());
        let base_name = normalize_process_name(process_name);
        rect.color = palette_color_for_key(theme, &base_name);
    }
}

fn apply_memory_heatmap(rects: &mut [ColoredTreemapRect], total_memory: u64, theme: &Theme) {
    for rect in rects.iter_mut() {
        rect.color = memory_color(rect.value, total_memory, theme);
    }
}

fn memory_color(memory_bytes: u64, total_memory: u64, theme: &Theme) -> Color {
    if total_memory == 0 {
        return theme.heat_colors[0];
    }
    let pct = memory_bytes as f64 / total_memory as f64;
    if pct > 0.50 {
        theme.heat_colors[4]
    } else if pct > 0.20 {
        theme.heat_colors[3]
    } else if pct > 0.05 {
        theme.heat_colors[2]
    } else if pct > 0.0 {
        theme.heat_colors[1]
    } else {
        theme.heat_colors[0]
    }
}

fn apply_cpu_heatmap(rects: &mut [ColoredTreemapRect], process_tree: &ProcessTree, theme: &Theme) {
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
    } else if cpu_percent > 50.0 {
        theme.heat_colors[3]
    } else if cpu_percent > 20.0 {
        theme.heat_colors[2]
    } else if cpu_percent > 0.0 {
        theme.heat_colors[1]
    } else {
        theme.heat_colors[0]
    }
}

fn apply_user_colors(rects: &mut [ColoredTreemapRect], process_tree: &ProcessTree, theme: &Theme) {
    apply_hash_colors(rects, process_tree, theme, |p| {
        p.user_id.clone().unwrap_or_default()
    });
}

fn apply_group_colors(rects: &mut [ColoredTreemapRect], process_tree: &ProcessTree, theme: &Theme) {
    apply_hash_colors(rects, process_tree, theme, |p| {
        p.group_id.clone().unwrap_or_default()
    });
}

fn apply_hash_colors(
    rects: &mut [ColoredTreemapRect],
    process_tree: &ProcessTree,
    theme: &Theme,
    key_fn: impl Fn(&crate::system::process::ProcessInfo) -> String,
) {
    let mut color_map: std::collections::HashMap<String, Color> = std::collections::HashMap::new();
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

fn normalize_process_name(name: &str) -> String {
    let lowered = name.trim().to_lowercase();
    if lowered.is_empty() {
        return "unknown".to_string();
    }

    let no_parens = lowered.split('(').next().unwrap_or("").trim().to_string();

    let no_suffix = strip_known_suffixes(&no_parens);
    let no_variant = no_suffix
        .split_once(" - ")
        .map(|(head, _)| head)
        .unwrap_or(&no_suffix)
        .trim();

    let head = no_variant
        .split('.')
        .next()
        .unwrap_or(no_variant)
        .split_whitespace()
        .next()
        .unwrap_or(no_variant)
        .trim();

    if head.is_empty() {
        "unknown".to_string()
    } else {
        head.to_string()
    }
}

fn strip_known_suffixes(name: &str) -> String {
    let mut value = name.trim().to_string();
    loop {
        let mut changed = false;
        for suffix in [
            " helper",
            " renderer",
            " gpu process",
            " gpu",
            " utility process",
            " utility",
            " crashpad",
            " broker",
            " service",
        ] {
            if value.ends_with(suffix) {
                value.truncate(value.len().saturating_sub(suffix.len()));
                value = value.trim().to_string();
                changed = true;
                break;
            }
        }
        if !changed {
            break;
        }
    }
    value
}

fn palette_color_for_key(theme: &Theme, key: &str) -> Color {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    let idx = (hasher.finish() as usize) % theme.hash_palette.len();
    theme.hash_palette[idx]
}

fn apply_monochrome(rects: &mut [ColoredTreemapRect], total_memory: u64, theme: &Theme) {
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

fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

fn adapt_color(color: Color, support: ColorSupport) -> Color {
    match support {
        ColorSupport::Truecolor | ColorSupport::Auto => color,
        ColorSupport::Color256 => match color {
            Color::Rgb(r, g, b) => Color::Indexed(rgb_to_ansi256(r, g, b)),
            _ => color,
        },
        ColorSupport::Mono => match color {
            Color::Rgb(r, g, b) => {
                let luminance = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
                if luminance > 128.0 {
                    Color::White
                } else {
                    Color::Black
                }
            }
            Color::White | Color::Black | Color::Gray | Color::DarkGray => color,
            _ => Color::White,
        },
    }
}

fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    let r = (r as f32 / 255.0 * 5.0).round() as u8;
    let g = (g as f32 / 255.0 * 5.0).round() as u8;
    let b = (b as f32 / 255.0 * 5.0).round() as u8;
    16 + 36 * r + 6 * g + b
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
            group_name: None,
            priority: None,
            io_stats: None,
        }
    }

    fn make_tree(procs: Vec<ProcessInfo>) -> ProcessTree {
        let mut processes = std::collections::HashMap::new();
        for p in procs {
            processes.insert(p.pid, p);
        }
        ProcessTree { processes }
    }

    #[test]
    fn color_mode_cycles() {
        let mut mode = ColorMode::ByName;
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByMemory);
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByCpu);
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByUser);
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByGroup);
        mode = mode.next();
        assert_eq!(mode, ColorMode::Monochrome);
        mode = mode.next();
        assert_eq!(mode, ColorMode::ByName);
    }

    #[test]
    fn color_mode_labels() {
        assert_eq!(ColorMode::ByName.label(), "Name");
        assert_eq!(ColorMode::ByMemory.label(), "Memory");
        assert_eq!(ColorMode::ByCpu.label(), "CPU");
        assert_eq!(ColorMode::ByUser.label(), "User");
        assert_eq!(ColorMode::ByGroup.label(), "Group");
        assert_eq!(ColorMode::Monochrome.label(), "Mono");
    }

    #[test]
    fn color_mode_from_config() {
        assert_eq!(ColorMode::from_str_config("name"), ColorMode::ByName);
        assert_eq!(ColorMode::from_str_config("memory"), ColorMode::ByMemory);
        assert_eq!(ColorMode::from_str_config("cpu"), ColorMode::ByCpu);
        assert_eq!(ColorMode::from_str_config("user"), ColorMode::ByUser);
        assert_eq!(ColorMode::from_str_config("group"), ColorMode::ByGroup);
        assert_eq!(ColorMode::from_str_config("mono"), ColorMode::Monochrome);
        assert_eq!(ColorMode::from_str_config("unknown"), ColorMode::ByMemory);
    }

    #[test]
    fn name_colors_group_related_processes() {
        let heat = HeatOverrides {
            low: "#475569".to_string(),
            mid: "#f97316".to_string(),
            high: "#ec4899".to_string(),
        };
        let theme = Theme::from_config("vivid", &heat, ColorSupport::Truecolor);
        let tree = make_tree(vec![
            ProcessInfo {
                name: "Brave Browser Helper".to_string(),
                ..make_process(1, 100, 0.0)
            },
            ProcessInfo {
                name: "Brave Browser Renderer".to_string(),
                ..make_process(2, 100, 0.0)
            },
            ProcessInfo {
                name: "Code".to_string(),
                ..make_process(3, 100, 0.0)
            },
        ]);
        let rects = vec![make_rect(1, 100), make_rect(2, 100), make_rect(3, 100)];
        let colored = colorize_rects(
            &rects,
            &tree,
            300,
            ColorMode::ByName,
            &theme,
            ColorSupport::Truecolor,
        );

        assert_eq!(colored[0].color, colored[1].color);
        assert_ne!(colored[0].color, colored[2].color);
    }

    #[test]
    fn name_normalization_collapses_suffixes_and_domains() {
        assert_eq!(normalize_process_name("Brave Browser Helper"), "brave");
        assert_eq!(normalize_process_name("Brave Browser Renderer"), "brave");
        assert_eq!(normalize_process_name("com.apple.WebKit.GPU"), "com");
        assert_eq!(normalize_process_name("Code - Helper (Renderer)"), "code");
    }

    #[test]
    fn memory_heatmap_assigns_colors() {
        let heat = HeatOverrides {
            low: "#2d5a27".to_string(),
            mid: "#b5890a".to_string(),
            high: "#a12e2e".to_string(),
        };
        let theme = Theme::from_config("dark", &heat, ColorSupport::Truecolor);
        let rects = vec![
            make_rect(1, 600_000_000),
            make_rect(2, 50_000_000),
            make_rect(3, 10_000_000),
        ];
        let colored = colorize_rects(
            &rects,
            &make_tree(vec![]),
            1_024_000_000,
            ColorMode::ByMemory,
            &theme,
            ColorSupport::Truecolor,
        );
        assert_eq!(colored[0].color, theme.heat_colors[4]);
    }

    #[test]
    fn user_colors_same_user_same_color() {
        let heat = HeatOverrides {
            low: "#2d5a27".to_string(),
            mid: "#b5890a".to_string(),
            high: "#a12e2e".to_string(),
        };
        let theme = Theme::from_config("dark", &heat, ColorSupport::Truecolor);
        let procs = vec![make_process(1, 100, 10.0), make_process(4, 100, 10.0)];
        let tree = make_tree(procs);
        let rects = vec![make_rect(1, 100), make_rect(4, 100)];
        let colored = colorize_rects(
            &rects,
            &tree,
            200,
            ColorMode::ByUser,
            &theme,
            ColorSupport::Truecolor,
        );
        assert_eq!(colored[0].color, colored[1].color);
    }

    #[test]
    fn memory_color_threshold_boundaries() {
        let heat = HeatOverrides {
            low: "#475569".to_string(),
            mid: "#f97316".to_string(),
            high: "#ec4899".to_string(),
        };
        let theme = Theme::from_config("vivid", &heat, ColorSupport::Truecolor);

        assert_eq!(memory_color(0, 100, &theme), theme.heat_colors[0]);
        assert_eq!(memory_color(5, 100, &theme), theme.heat_colors[1]);
        assert_eq!(memory_color(6, 100, &theme), theme.heat_colors[2]);
        assert_eq!(memory_color(20, 100, &theme), theme.heat_colors[2]);
        assert_eq!(memory_color(21, 100, &theme), theme.heat_colors[3]);
        assert_eq!(memory_color(50, 100, &theme), theme.heat_colors[3]);
        assert_eq!(memory_color(51, 100, &theme), theme.heat_colors[4]);
    }

    #[test]
    fn other_group_is_always_neutral() {
        let heat = HeatOverrides {
            low: "#475569".to_string(),
            mid: "#f97316".to_string(),
            high: "#ec4899".to_string(),
        };
        let theme = Theme::from_config("vivid", &heat, ColorSupport::Truecolor);
        let tree = make_tree(vec![make_process(1, 100, 40.0), make_process(2, 120, 90.0)]);
        let rects = vec![make_rect(0, 900), make_rect(2, 120)];

        for mode in [
            ColorMode::ByName,
            ColorMode::ByMemory,
            ColorMode::ByCpu,
            ColorMode::ByUser,
            ColorMode::ByGroup,
            ColorMode::Monochrome,
        ] {
            let colored =
                colorize_rects(&rects, &tree, 1_000, mode, &theme, ColorSupport::Truecolor);
            assert_eq!(colored[0].id, 0);
            assert_eq!(colored[0].color, theme.other_group_bg);
        }
    }
}
