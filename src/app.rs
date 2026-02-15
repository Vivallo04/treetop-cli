use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use crate::action::{Action, Direction};
use crate::config::{Config, parse_key};
use crate::format::format_bytes;
use crate::system::collector::Collector;
use crate::system::history::HistoryStore;
use crate::system::kill::{KillResult, kill_process};
use crate::system::snapshot::SystemSnapshot;
use crate::treemap::node::{LayoutRect, TreemapItem, TreemapRect};
use crate::ui::theme::{
    BorderStyle, ColorMode, ColorSupport, HeatOverrides, Theme, resolve_color_support,
};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
    Help,
}

#[derive(Debug, Clone)]
pub struct ResolvedKeybinds {
    pub quit: KeyCode,
    pub filter: KeyCode,
    pub kill: KeyCode,
    pub force_kill: KeyCode,
    pub cycle_color: KeyCode,
    pub cycle_theme: KeyCode,
    pub toggle_detail: KeyCode,
    pub zoom_in: KeyCode,
    pub zoom_out: KeyCode,
    pub help: KeyCode,
    pub cycle_sort: KeyCode,
    pub refresh: KeyCode,
}

impl ResolvedKeybinds {
    pub fn from_config(kb: &crate::config::KeybindsConfig) -> Self {
        Self {
            quit: parse_key(&kb.quit).unwrap_or(KeyCode::Char('q')),
            filter: parse_key(&kb.filter).unwrap_or(KeyCode::Char('/')),
            kill: parse_key(&kb.kill).unwrap_or(KeyCode::Char('k')),
            force_kill: parse_key(&kb.force_kill).unwrap_or(KeyCode::Char('K')),
            cycle_color: parse_key(&kb.cycle_color).unwrap_or(KeyCode::Char('c')),
            cycle_theme: parse_key(&kb.cycle_theme).unwrap_or(KeyCode::Char('t')),
            toggle_detail: parse_key(&kb.toggle_detail).unwrap_or(KeyCode::Char('d')),
            zoom_in: parse_key(&kb.zoom_in).unwrap_or(KeyCode::Enter),
            zoom_out: parse_key(&kb.zoom_out).unwrap_or(KeyCode::Esc),
            help: parse_key(&kb.help).unwrap_or(KeyCode::Char('?')),
            cycle_sort: parse_key(&kb.cycle_sort).unwrap_or(KeyCode::Char('s')),
            refresh: parse_key(&kb.refresh).unwrap_or(KeyCode::Char('r')),
        }
    }

    /// Returns (key_label, description) pairs for all configurable keybinds.
    pub fn help_entries(&self) -> Vec<(String, &'static str)> {
        let mut entries = vec![
            (key_label(self.quit), "Quit"),
            (key_label(self.filter), "Filter processes"),
            (key_label(self.kill), "Kill process (SIGTERM)"),
            (key_label(self.force_kill), "Force kill (SIGKILL)"),
            (key_label(self.cycle_color), "Cycle color mode"),
            (key_label(self.cycle_theme), "Cycle theme"),
            (key_label(self.toggle_detail), "Toggle detail panel"),
            (key_label(self.zoom_in), "Zoom in"),
            (key_label(self.zoom_out), "Zoom out"),
            (key_label(self.help), "Toggle help"),
            (key_label(self.cycle_sort), "Cycle sort mode"),
            (key_label(self.refresh), "Refresh data"),
        ];
        entries.push(("↑↓←→".to_string(), "Navigate"));
        entries.push(("Ctrl+C".to_string(), "Quit (always)"));
        entries
    }
}

fn key_label(code: KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Bksp".to_string(),
        KeyCode::Delete => "Del".to_string(),
        _ => "?".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Memory,
    Cpu,
    Name,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            SortMode::Memory => SortMode::Cpu,
            SortMode::Cpu => SortMode::Name,
            SortMode::Name => SortMode::Memory,
        }
    }

    #[allow(dead_code)] // Used in Step 7 (statusbar sort label)
    pub fn label(self) -> &'static str {
        match self {
            SortMode::Memory => "Memory",
            SortMode::Cpu => "CPU",
            SortMode::Name => "Name",
        }
    }

    pub fn from_str_config(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cpu" => SortMode::Cpu,
            "name" => SortMode::Name,
            _ => SortMode::Memory,
        }
    }
}

pub struct App {
    pub running: bool,
    pub collector: Collector,
    pub snapshot: SystemSnapshot,
    pub layout_rects: Vec<TreemapRect>,
    pub selected_index: usize,
    pub input_mode: InputMode,
    pub filter_text: String,
    pub show_detail_panel: bool,
    pub color_mode: ColorMode,
    pub theme: Theme,
    pub color_support: ColorSupport,
    pub border_style: BorderStyle,
    pub status_message: Option<(String, Instant)>,
    pub treemap_area: Option<Rect>,
    pub min_rect_width: u16,
    pub min_rect_height: u16,
    pub zoom_stack: Vec<u32>,
    pub history: HistoryStore,
    pub cpu_history: VecDeque<u64>,
    cpu_history_capacity: usize,
    heat_overrides: HeatOverrides,
    group_threshold: f64,
    subtree_sizes: HashMap<u32, u64>,
    prev_layout_rects: Vec<TreemapRect>,
    animation_frame: u8,
    anim_frames: u8,
    max_visible_procs: usize,
    needs_relayout: bool,
    pub sort_mode: SortMode,
    pub keybinds: ResolvedKeybinds,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut collector = Collector::new();
        let snapshot = collector.refresh();

        let show_detail_panel = config.general.show_detail_panel;
        let color_support = resolve_color_support(&config.general.color_support);
        let heat_overrides = HeatOverrides::from_config(&config.colors);
        let mut color_mode = ColorMode::from_str_config(&config.general.default_color_mode);
        if color_support == ColorSupport::Mono {
            color_mode = ColorMode::Monochrome;
        }
        let theme = Theme::from_config(&config.colors.theme, &heat_overrides, color_support);
        let border_style = BorderStyle::from_config_str(&config.treemap.border_style);
        let min_rect_width = config.treemap.min_rect_width;
        let min_rect_height = config.treemap.min_rect_height;
        let max_visible_procs = config.treemap.max_visible_procs;
        let anim_frames = config.treemap.animation_frames;
        let sparkline_length = config.general.sparkline_length;
        let group_threshold = config.treemap.group_threshold;
        let sort_mode = SortMode::from_str_config(&config.general.default_sort);
        let keybinds = ResolvedKeybinds::from_config(&config.keybinds);

        App {
            running: true,
            collector,
            snapshot,
            layout_rects: Vec::new(),
            selected_index: 0,
            input_mode: InputMode::Normal,
            filter_text: String::new(),
            show_detail_panel,
            color_mode,
            theme,
            color_support,
            border_style,
            status_message: None,
            treemap_area: None,
            min_rect_width,
            min_rect_height,
            zoom_stack: Vec::new(),
            history: HistoryStore::new(sparkline_length),
            cpu_history: VecDeque::with_capacity(sparkline_length),
            cpu_history_capacity: sparkline_length,
            heat_overrides,
            group_threshold,
            subtree_sizes: HashMap::new(),
            prev_layout_rects: Vec::new(),
            animation_frame: 0,
            anim_frames,
            max_visible_procs,
            needs_relayout: true,
            sort_mode,
            keybinds,
        }
    }

    pub fn refresh_data(&mut self) {
        self.snapshot = self.collector.refresh();
        self.needs_relayout = true;

        // Record system-level CPU history
        let cpu_val = (self.snapshot.cpu_usage_percent * 100.0) as u64;
        if self.cpu_history.len() == self.cpu_history_capacity {
            self.cpu_history.pop_front();
        }
        self.cpu_history.push_back(cpu_val);

        // Recompute subtree sizes
        self.subtree_sizes = self.snapshot.process_tree.all_subtree_sizes();

        // Record history for all processes
        for p in self.snapshot.process_tree.processes.values() {
            self.history.record(p.pid, p.memory_bytes, p.cpu_percent);
        }
        let alive: std::collections::HashSet<u32> = self
            .snapshot
            .process_tree
            .processes
            .keys()
            .copied()
            .collect();
        self.history.gc(&alive);

        // Validate zoom stack — remove PIDs that no longer exist
        self.zoom_stack
            .retain(|pid| self.snapshot.process_tree.processes.contains_key(pid));

        // Clear expired status messages (older than 3 seconds)
        if let Some((_, created)) = &self.status_message
            && created.elapsed().as_secs() >= 3
        {
            self.status_message = None;
        }
    }

    pub fn compute_layout(&mut self, width: u16, height: u16) {
        if !self.needs_relayout {
            return;
        }

        #[cfg(feature = "perf-tracing")]
        let _layout_span = tracing::debug_span!(
            "app.compute_layout",
            width = width,
            height = height,
            current_rects = self.layout_rects.len()
        )
        .entered();

        let filter_lower = self.filter_text.to_lowercase();

        // If zoomed, show only the children of the zoom target
        let source_pids: Option<Vec<u32>> = self.zoom_pid().and_then(|zpid| {
            self.snapshot
                .process_tree
                .processes
                .get(&zpid)
                .map(|p| p.children.clone())
        });

        let subtree = &self.subtree_sizes;

        #[cfg(feature = "perf-tracing")]
        let _build_items_span = tracing::debug_span!("app.compute_layout.build_items").entered();

        let mut items: Vec<TreemapItem> = if let Some(children) = &source_pids {
            children
                .iter()
                .filter_map(|pid| self.snapshot.process_tree.processes.get(pid))
                .filter(|p| {
                    let sz = subtree.get(&p.pid).copied().unwrap_or(p.memory_bytes);
                    sz > 0
                        && (filter_lower.is_empty()
                            || p.name.to_lowercase().contains(&filter_lower)
                            || p.command.to_lowercase().contains(&filter_lower))
                })
                .map(|p| TreemapItem {
                    pid: p.pid,
                    label: p.name.clone(),
                    value: subtree.get(&p.pid).copied().unwrap_or(p.memory_bytes),
                })
                .collect()
        } else {
            self.snapshot
                .process_tree
                .processes
                .values()
                .filter(|p| {
                    p.memory_bytes > 0
                        && (filter_lower.is_empty()
                            || p.name.to_lowercase().contains(&filter_lower)
                            || p.command.to_lowercase().contains(&filter_lower))
                })
                .map(|p| TreemapItem {
                    pid: p.pid,
                    label: p.name.clone(),
                    value: p.memory_bytes,
                })
                .collect()
        };

        #[cfg(feature = "perf-tracing")]
        drop(_build_items_span);

        #[cfg(feature = "perf-tracing")]
        let _group_span = tracing::debug_span!("app.compute_layout.grouping").entered();

        let total_value: u64 = items.iter().map(|i| i.value).sum();
        let mut other_count = 0usize;
        let mut other_value = 0u64;

        if total_value > 0 && self.group_threshold > 0.0 {
            let mut filtered = Vec::with_capacity(items.len());
            for item in items.into_iter() {
                let ratio = item.value as f64 / total_value as f64;
                if ratio < self.group_threshold {
                    other_count += 1;
                    other_value += item.value;
                } else {
                    filtered.push(item);
                }
            }
            items = filtered;
        }

        match self.sort_mode {
            SortMode::Memory => {
                items.sort_by(|a, b| b.value.cmp(&a.value));
            }
            SortMode::Cpu => {
                let cpu_map: HashMap<u32, f32> = self
                    .snapshot
                    .process_tree
                    .processes
                    .values()
                    .map(|p| (p.pid, p.cpu_percent))
                    .collect();
                items.sort_by(|a, b| {
                    let ca = cpu_map.get(&a.pid).copied().unwrap_or(0.0);
                    let cb = cpu_map.get(&b.pid).copied().unwrap_or(0.0);
                    cb.partial_cmp(&ca).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            SortMode::Name => {
                items.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
            }
        }

        if self.max_visible_procs > 0 && items.len() > self.max_visible_procs {
            let small_items = items.split_off(self.max_visible_procs);
            other_count += small_items.len();
            other_value += small_items.iter().map(|i| i.value).sum::<u64>();
        }

        if other_value > 0 {
            let max_visible_value = items.first().map(|i| i.value).unwrap_or(other_value);
            let capped_value = other_value.min(max_visible_value);
            items.push(TreemapItem {
                pid: 0,
                label: format!(
                    "Other ({} procs, {})",
                    other_count,
                    format_bytes(other_value)
                ),
                value: capped_value,
            });
        }

        #[cfg(feature = "perf-tracing")]
        drop(_group_span);

        #[cfg(feature = "perf-tracing")]
        let _sort_span = tracing::debug_span!("app.compute_layout.sort").entered();

        let bounds = LayoutRect::new(0.0, 0.0, width as f64, height as f64);

        // Save old layout for animation
        if !self.layout_rects.is_empty() {
            self.prev_layout_rects = self.layout_rects.clone();
            self.animation_frame = 1;
        }

        #[cfg(feature = "perf-tracing")]
        drop(_sort_span);

        #[cfg(feature = "perf-tracing")]
        let _squarify_span = tracing::debug_span!("app.compute_layout.squarify").entered();

        self.layout_rects = crate::treemap::algorithm::squarify_sorted(&items, &bounds);

        if self.selected_index >= self.layout_rects.len() && !self.layout_rects.is_empty() {
            self.selected_index = 0;
        }
        self.needs_relayout = false;
    }

    pub fn map_key(&self, key: KeyEvent) -> Action {
        // Ctrl+C always quits (hardwired safety)
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Action::Quit;
        }

        match self.input_mode {
            InputMode::Normal => self.map_key_normal(key),
            InputMode::Filter => self.map_key_filter(key),
            InputMode::Help => self.map_key_help(key),
        }
    }

    fn map_key_normal(&self, key: KeyEvent) -> Action {
        let code = key.code;
        let kb = &self.keybinds;

        // Arrow keys are hardwired (not configurable)
        if let KeyCode::Up = code {
            return Action::Navigate(Direction::Up);
        }
        if let KeyCode::Down = code {
            return Action::Navigate(Direction::Down);
        }
        if let KeyCode::Left = code {
            return Action::Navigate(Direction::Left);
        }
        if let KeyCode::Right = code {
            return Action::Navigate(Direction::Right);
        }

        if code == kb.quit {
            return Action::Quit;
        }
        if code == kb.filter {
            return Action::EnterFilterMode;
        }
        if code == kb.kill {
            return if let Some(pid) = self.selected_pid() {
                Action::Kill(pid)
            } else {
                Action::None
            };
        }
        if code == kb.force_kill {
            return if let Some(pid) = self.selected_pid() {
                Action::ForceKill(pid)
            } else {
                Action::None
            };
        }
        if code == kb.cycle_color {
            return Action::CycleColorMode;
        }
        if code == kb.cycle_theme {
            return Action::CycleTheme;
        }
        if code == kb.toggle_detail {
            return Action::ToggleDetailPanel;
        }
        if code == kb.zoom_in {
            return Action::ZoomIn;
        }
        if code == kb.zoom_out {
            return Action::ZoomOut;
        }
        if code == kb.help {
            return Action::ToggleHelp;
        }
        if code == kb.cycle_sort {
            return Action::CycleSortMode;
        }
        if code == kb.refresh {
            return Action::Refresh;
        }

        Action::None
    }

    fn map_key_help(&self, key: KeyEvent) -> Action {
        let code = key.code;
        // In help mode, only the help key and Esc dismiss, everything else is ignored
        if code == self.keybinds.help || code == KeyCode::Esc {
            return Action::ToggleHelp;
        }
        Action::None
    }

    fn map_key_filter(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => Action::ClearFilter,
            KeyCode::Enter => Action::ExitFilterMode,
            KeyCode::Backspace => {
                let mut text = self.filter_text.clone();
                text.pop();
                Action::UpdateFilter(text)
            }
            KeyCode::Char(c) => {
                let mut text = self.filter_text.clone();
                text.push(c);
                Action::UpdateFilter(text)
            }
            _ => Action::None,
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::Navigate(dir) => self.navigate(dir),
            Action::EnterFilterMode => {
                self.input_mode = InputMode::Filter;
            }
            Action::ExitFilterMode => {
                self.input_mode = InputMode::Normal;
            }
            Action::ClearFilter => {
                self.filter_text.clear();
                self.input_mode = InputMode::Normal;
                self.needs_relayout = true;
            }
            Action::UpdateFilter(text) => {
                self.filter_text = text;
                self.needs_relayout = true;
            }
            Action::CycleColorMode => {
                if self.color_support == ColorSupport::Mono {
                    self.color_mode = ColorMode::Monochrome;
                } else {
                    self.color_mode = self.color_mode.next();
                }
                self.needs_relayout = true;
            }
            Action::CycleTheme => {
                self.theme = self.theme.next(&self.heat_overrides, self.color_support);
                self.needs_relayout = true;
            }
            Action::ToggleDetailPanel => {
                self.show_detail_panel = !self.show_detail_panel;
                self.needs_relayout = true;
            }
            Action::ZoomIn => self.zoom_in(),
            Action::ZoomOut => self.zoom_out(),
            Action::SelectAt(col, row) => {
                self.select_at(col, row);
            }
            Action::Kill(pid) => {
                if pid != 0 {
                    let result = kill_process(self.collector.system(), pid, sysinfo::Signal::Term);
                    self.set_kill_status(result);
                }
            }
            Action::ForceKill(pid) => {
                if pid != 0 {
                    let result = kill_process(self.collector.system(), pid, sysinfo::Signal::Kill);
                    self.set_kill_status(result);
                }
            }
            Action::ToggleHelp => {
                self.input_mode = if self.input_mode == InputMode::Help {
                    InputMode::Normal
                } else {
                    InputMode::Help
                };
            }
            Action::CycleSortMode => {
                self.sort_mode = self.sort_mode.next();
                self.needs_relayout = true;
            }
            Action::Refresh => {
                self.refresh_data();
            }
            Action::None => {}
        }
    }

    fn navigate(&mut self, direction: Direction) {
        if self.layout_rects.is_empty() {
            return;
        }

        let current = &self.layout_rects[self.selected_index].rect;
        let cx = current.x + current.width / 2.0;
        let cy = current.y + current.height / 2.0;

        let mut best_index = self.selected_index;
        let mut best_dist = f64::MAX;

        for (i, r) in self.layout_rects.iter().enumerate() {
            if i == self.selected_index {
                continue;
            }
            let rx = r.rect.x + r.rect.width / 2.0;
            let ry = r.rect.y + r.rect.height / 2.0;
            let dx = rx - cx;
            let dy = ry - cy;

            let in_direction = match direction {
                Direction::Up => dy < -0.1,
                Direction::Down => dy > 0.1,
                Direction::Left => dx < -0.1,
                Direction::Right => dx > 0.1,
            };
            if !in_direction {
                continue;
            }

            let dist = match direction {
                Direction::Up | Direction::Down => dy.abs() + dx.abs() * 0.5,
                Direction::Left | Direction::Right => dx.abs() + dy.abs() * 0.5,
            };
            if dist < best_dist {
                best_dist = dist;
                best_index = i;
            }
        }
        self.selected_index = best_index;
    }

    fn select_at(&mut self, col: u16, row: u16) {
        let area = match self.treemap_area {
            Some(a) => a,
            None => return,
        };

        if col < area.x || col >= area.x + area.width || row < area.y || row >= area.y + area.height
        {
            return;
        }

        let local_col = (col - area.x) as f64;
        let local_row = (row - area.y) as f64;

        for (i, r) in self.layout_rects.iter().enumerate() {
            if local_col >= r.rect.x
                && local_col < r.rect.x + r.rect.width
                && local_row >= r.rect.y
                && local_row < r.rect.y + r.rect.height
            {
                self.selected_index = i;
                return;
            }
        }
    }

    pub fn selected_pid(&self) -> Option<u32> {
        self.layout_rects.get(self.selected_index).map(|r| r.pid)
    }

    pub fn selected_process(&self) -> Option<&crate::system::process::ProcessInfo> {
        self.selected_pid()
            .and_then(|pid| self.snapshot.process_tree.processes.get(&pid))
    }

    pub fn show_help(&self) -> bool {
        self.input_mode == InputMode::Help
    }

    pub fn help_entries(&self) -> Vec<(String, &'static str)> {
        self.keybinds.help_entries()
    }

    fn set_kill_status(&mut self, result: KillResult) {
        let msg = match result {
            KillResult::Success(pid, signal) => format!("Sent {signal} to PID {pid}"),
            KillResult::Failed(err) => err,
            KillResult::NotFound(pid) => format!("Process {pid} not found"),
        };
        self.status_message = Some((msg, Instant::now()));
    }

    pub fn on_resize(&mut self) {
        self.needs_relayout = true;
    }

    pub fn zoom_pid(&self) -> Option<u32> {
        self.zoom_stack.last().copied()
    }

    pub fn is_zoomed(&self) -> bool {
        !self.zoom_stack.is_empty()
    }

    fn zoom_in(&mut self) {
        let pid = match self.selected_pid() {
            Some(pid) if pid != 0 => pid,
            _ => return,
        };
        // Only zoom if the process has children
        if let Some(process) = self.snapshot.process_tree.processes.get(&pid)
            && !process.children.is_empty()
        {
            self.zoom_stack.push(pid);
            self.selected_index = 0;
            self.needs_relayout = true;
        }
    }

    fn zoom_out(&mut self) {
        if self.zoom_stack.pop().is_some() {
            self.selected_index = 0;
            self.needs_relayout = true;
        }
    }

    pub fn zoom_breadcrumbs(&self) -> Vec<(u32, String)> {
        self.zoom_stack
            .iter()
            .filter_map(|&pid| {
                self.snapshot
                    .process_tree
                    .processes
                    .get(&pid)
                    .map(|p| (pid, p.name.clone()))
            })
            .collect()
    }

    pub fn is_animating(&self) -> bool {
        self.animation_frame > 0 && self.animation_frame <= self.anim_frames
    }

    pub fn tick_animation(&mut self) {
        if self.is_animating() {
            self.animation_frame += 1;
            if self.animation_frame > self.anim_frames {
                self.animation_frame = 0;
                self.prev_layout_rects.clear();
            }
        }
    }

    pub fn display_rects(&self) -> Vec<TreemapRect> {
        if !self.is_animating() || self.prev_layout_rects.is_empty() {
            return self.layout_rects.clone();
        }

        let t = self.animation_frame as f64 / self.anim_frames as f64;

        self.layout_rects
            .iter()
            .map(|new_rect| {
                // Find matching old rect by pid
                let old = self
                    .prev_layout_rects
                    .iter()
                    .find(|old| old.pid == new_rect.pid);

                match old {
                    Some(old_rect) => TreemapRect {
                        rect: old_rect.rect.lerp(&new_rect.rect, t),
                        pid: new_rect.pid,
                        label: new_rect.label.clone(),
                        value: new_rect.value,
                    },
                    None => new_rect.clone(), // New rect, no transition
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::process::{ProcessInfo, ProcessState, ProcessTree};
    use crate::system::snapshot::SystemSnapshot;
    use std::collections::HashMap;

    fn make_test_process(pid: u32, name: &str, memory: u64, cpu: f32) -> ProcessInfo {
        ProcessInfo {
            pid,
            ppid: 0,
            name: name.to_string(),
            command: format!("{name} --flag"),
            memory_bytes: memory,
            cpu_percent: cpu,
            user_id: Some("user".to_string()),
            group_id: Some("group".to_string()),
            status: ProcessState::Running,
            children: Vec::new(),
            group_name: None,
            priority: None,
            io_stats: None,
        }
    }

    fn make_test_app_with_processes(procs: Vec<ProcessInfo>, sort_mode: SortMode) -> App {
        let mut processes = HashMap::new();
        for p in procs {
            processes.insert(p.pid, p);
        }
        let snapshot = SystemSnapshot {
            cpu_usage_percent: 10.0,
            memory_total: 1_000_000_000,
            memory_used: 500_000_000,
            swap_total: 0,
            swap_used: 0,
            cpu_per_core: vec![],
            load_average: [0.0; 3],
            process_tree: ProcessTree { processes },
        };

        let mut app = App {
            running: true,
            collector: Collector::new(),
            snapshot,
            layout_rects: Vec::new(),
            selected_index: 0,
            input_mode: InputMode::Normal,
            filter_text: String::new(),
            show_detail_panel: false,
            color_mode: ColorMode::ByMemory,
            theme: Theme::from_config(
                "vivid",
                &HeatOverrides {
                    low: String::new(),
                    mid: String::new(),
                    high: String::new(),
                },
                ColorSupport::Color256,
            ),
            color_support: ColorSupport::Color256,
            border_style: BorderStyle::Rounded,
            status_message: None,
            treemap_area: None,
            min_rect_width: 4,
            min_rect_height: 2,
            zoom_stack: Vec::new(),
            history: HistoryStore::new(20),
            cpu_history: VecDeque::new(),
            cpu_history_capacity: 20,
            heat_overrides: HeatOverrides {
                low: String::new(),
                mid: String::new(),
                high: String::new(),
            },
            group_threshold: 0.0,
            subtree_sizes: HashMap::new(),
            prev_layout_rects: Vec::new(),
            animation_frame: 0,
            anim_frames: 5,
            max_visible_procs: 0,
            needs_relayout: true,
            sort_mode,
            keybinds: ResolvedKeybinds::from_config(&crate::config::KeybindsConfig::default()),
        };
        app.compute_layout(100, 50);
        app
    }

    #[test]
    fn sort_mode_cycles_through_all_variants() {
        let mode = SortMode::Memory;
        assert_eq!(mode.next(), SortMode::Cpu);
        assert_eq!(mode.next().next(), SortMode::Name);
        assert_eq!(mode.next().next().next(), SortMode::Memory);
    }

    #[test]
    fn compute_layout_cpu_sort_orders_by_cpu_descending() {
        // Process with less memory but higher CPU should come first in CPU sort
        let procs = vec![
            make_test_process(1, "low_cpu", 500_000_000, 5.0),
            make_test_process(2, "high_cpu", 100_000_000, 90.0),
            make_test_process(3, "mid_cpu", 300_000_000, 50.0),
        ];
        let app = make_test_app_with_processes(procs, SortMode::Cpu);

        assert!(!app.layout_rects.is_empty());
        let labels: Vec<&str> = app.layout_rects.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["high_cpu", "mid_cpu", "low_cpu"]);
    }

    #[test]
    fn compute_layout_name_sort_orders_alphabetically() {
        let procs = vec![
            make_test_process(1, "Zebra", 100_000, 1.0),
            make_test_process(2, "alpha", 200_000, 2.0),
            make_test_process(3, "Beta", 300_000, 3.0),
        ];
        let app = make_test_app_with_processes(procs, SortMode::Name);

        assert!(!app.layout_rects.is_empty());
        let labels: Vec<&str> = app.layout_rects.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, vec!["alpha", "Beta", "Zebra"]);
    }

    #[test]
    fn dispatch_cycle_sort_advances_mode() {
        let procs = vec![make_test_process(1, "test", 100_000, 1.0)];
        let mut app = make_test_app_with_processes(procs, SortMode::Memory);

        assert_eq!(app.sort_mode, SortMode::Memory);
        app.dispatch(Action::CycleSortMode);
        assert_eq!(app.sort_mode, SortMode::Cpu);
        app.dispatch(Action::CycleSortMode);
        assert_eq!(app.sort_mode, SortMode::Name);
        app.dispatch(Action::CycleSortMode);
        assert_eq!(app.sort_mode, SortMode::Memory);
    }

    #[test]
    fn default_keybinds_match_original_behavior() {
        let procs = vec![make_test_process(1, "test", 100_000, 1.0)];
        let app = make_test_app_with_processes(procs, SortMode::Memory);

        // Default 'q' key should map to Quit
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::Quit);

        // Default '/' key should map to EnterFilterMode
        let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::EnterFilterMode);

        // Default 's' should map to CycleSortMode
        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::CycleSortMode);

        // Default '?' should map to ToggleHelp
        let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::ToggleHelp);

        // Ctrl+C always quits
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(app.map_key(key), Action::Quit);

        // Arrow keys stay hardwired
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::Navigate(Direction::Up));
    }

    #[test]
    fn custom_keybind_remap_works() {
        let procs = vec![make_test_process(1, "test", 100_000, 1.0)];
        let mut app = make_test_app_with_processes(procs, SortMode::Memory);

        // Remap quit to 'x'
        app.keybinds.quit = KeyCode::Char('x');

        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::Quit);

        // 'q' should now do nothing
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::None);
    }

    #[test]
    fn help_mode_blocks_other_keys() {
        let procs = vec![make_test_process(1, "test", 100_000, 1.0)];
        let mut app = make_test_app_with_processes(procs, SortMode::Memory);

        // Enter help mode
        app.dispatch(Action::ToggleHelp);
        assert_eq!(app.input_mode, InputMode::Help);
        assert!(app.show_help());

        // Normal keys should be blocked in help mode
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::None);

        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::None);

        // But help key dismisses
        let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::ToggleHelp);

        // Esc also dismisses
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(app.map_key(key), Action::ToggleHelp);

        // Ctrl+C still works (safety)
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(app.map_key(key), Action::Quit);

        // Toggle back
        app.dispatch(Action::ToggleHelp);
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app.show_help());
    }
}
