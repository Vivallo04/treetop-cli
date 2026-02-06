use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;

use crate::action::{Action, Direction};
use crate::config::Config;
use crate::system::collector::Collector;
use crate::system::history::HistoryStore;
use crate::system::kill::{kill_process, KillResult};
use crate::system::snapshot::SystemSnapshot;
use crate::treemap::algorithm::squarify;
use crate::treemap::color::{apply_color_mode, ColorMode, Theme};
use crate::treemap::node::{LayoutRect, TreemapItem, TreemapRect};
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
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
    pub status_message: Option<(String, Instant)>,
    pub treemap_area: Option<Rect>,
    pub min_rect_width: u16,
    pub min_rect_height: u16,
    pub zoom_stack: Vec<u32>,
    pub history: HistoryStore,
    pub cpu_history: VecDeque<u64>,
    cpu_history_capacity: usize,
    subtree_sizes: HashMap<u32, u64>,
    prev_layout_rects: Vec<TreemapRect>,
    animation_frame: u8,
    anim_frames: u8,
    max_visible_procs: usize,
    needs_relayout: bool,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut collector = Collector::new();
        let snapshot = collector.refresh();

        let show_detail_panel = config.general.show_detail_panel;
        let color_mode = ColorMode::from_str_config(&config.general.default_color_mode);
        let theme = Theme::from_config(&config.colors.theme);
        let min_rect_width = config.treemap.min_rect_width;
        let min_rect_height = config.treemap.min_rect_height;
        let max_visible_procs = config.treemap.max_visible_procs;
        let anim_frames = config.treemap.animation_frames;
        let sparkline_length = config.general.sparkline_length;

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
            status_message: None,
            treemap_area: None,
            min_rect_width,
            min_rect_height,
            zoom_stack: Vec::new(),
            history: HistoryStore::new(sparkline_length),
            cpu_history: VecDeque::with_capacity(sparkline_length),
            cpu_history_capacity: sparkline_length,
            subtree_sizes: HashMap::new(),
            prev_layout_rects: Vec::new(),
            animation_frame: 0,
            anim_frames,
            max_visible_procs,
            needs_relayout: true,
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
                    id: p.pid,
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
                    id: p.pid,
                    label: p.name.clone(),
                    value: p.memory_bytes,
                })
                .collect()
        };

        items.sort_by(|a, b| b.value.cmp(&a.value));

        // Cap visible processes and group the rest into "Other"
        if self.max_visible_procs > 0 && items.len() > self.max_visible_procs {
            let small_items = items.split_off(self.max_visible_procs);
            let small_sum: u64 = small_items.iter().map(|i| i.value).sum();
            if small_sum > 0 {
                // Cap visual size to largest visible process so "Other" never dominates
                let max_visible_value = items.first().map(|i| i.value).unwrap_or(0);
                let capped_value = small_sum.min(max_visible_value);
                items.push(TreemapItem {
                    id: 0,
                    label: format!(
                        "Other ({} procs, {})",
                        small_items.len(),
                        format_bytes(small_sum)
                    ),
                    value: capped_value,
                });
            }
        }

        let bounds = LayoutRect::new(0.0, 0.0, width as f64, height as f64);

        // Save old layout for animation
        if !self.layout_rects.is_empty() {
            self.prev_layout_rects = self.layout_rects.clone();
            self.animation_frame = 1;
        }

        self.layout_rects = squarify(&items, &bounds);
        apply_color_mode(
            &mut self.layout_rects,
            self.color_mode,
            &self.snapshot.process_tree,
            self.snapshot.memory_total,
            &self.theme,
        );

        if self.selected_index >= self.layout_rects.len() && !self.layout_rects.is_empty() {
            self.selected_index = 0;
        }
        self.needs_relayout = false;
    }

    pub fn map_key(&self, key: KeyEvent) -> Action {
        match self.input_mode {
            InputMode::Normal => self.map_key_normal(key),
            InputMode::Filter => self.map_key_filter(key),
        }
    }

    fn map_key_normal(&self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,
            KeyCode::Char('/') => Action::EnterFilterMode,
            KeyCode::Char('k') => {
                if let Some(pid) = self.selected_pid() {
                    Action::Kill(pid)
                } else {
                    Action::None
                }
            }
            KeyCode::Char('K') => {
                if let Some(pid) = self.selected_pid() {
                    Action::ForceKill(pid)
                } else {
                    Action::None
                }
            }
            KeyCode::Char('c') => Action::CycleColorMode,
            KeyCode::Char('t') => Action::CycleTheme,
            KeyCode::Char('d') => Action::ToggleDetailPanel,
            KeyCode::Enter => Action::ZoomIn,
            KeyCode::Esc => Action::ZoomOut,
            KeyCode::Up => Action::Navigate(Direction::Up),
            KeyCode::Down => Action::Navigate(Direction::Down),
            KeyCode::Left => Action::Navigate(Direction::Left),
            KeyCode::Right => Action::Navigate(Direction::Right),
            _ => Action::None,
        }
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
                self.color_mode = self.color_mode.next();
                self.needs_relayout = true;
            }
            Action::CycleTheme => {
                self.theme = self.theme.next();
                self.needs_relayout = true;
            }
            Action::ToggleDetailPanel => {
                self.show_detail_panel = !self.show_detail_panel;
                self.needs_relayout = true;
            }
            Action::ZoomIn => self.zoom_in(),
            Action::ZoomOut => self.zoom_out(),
            Action::Refresh => {
                self.refresh_data();
            }
            Action::SelectAt(col, row) => {
                self.select_at(col, row);
            }
            Action::Kill(pid) => {
                let result =
                    kill_process(self.collector.system(), pid, sysinfo::Signal::Term);
                self.set_kill_status(result);
            }
            Action::ForceKill(pid) => {
                let result =
                    kill_process(self.collector.system(), pid, sysinfo::Signal::Kill);
                self.set_kill_status(result);
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
        self.layout_rects.get(self.selected_index).map(|r| r.id)
    }

    pub fn selected_process(&self) -> Option<&crate::system::process::ProcessInfo> {
        self.selected_pid()
            .and_then(|pid| self.snapshot.process_tree.processes.get(&pid))
    }

    fn set_kill_status(&mut self, result: KillResult) {
        let msg = match result {
            KillResult::Success(pid, signal) => format!("Sent {signal} to PID {pid}"),
            KillResult::Failed(_, err) => err,
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
                // Find matching old rect by id
                let old = self
                    .prev_layout_rects
                    .iter()
                    .find(|old| old.id == new_rect.id);

                match old {
                    Some(old_rect) => TreemapRect {
                        rect: old_rect.rect.lerp(&new_rect.rect, t),
                        id: new_rect.id,
                        label: new_rect.label.clone(),
                        value: new_rect.value,
                        color: new_rect.color,
                    },
                    None => new_rect.clone(), // New rect, no transition
                }
            })
            .collect()
    }
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
