use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::system::collector::Collector;
use crate::system::snapshot::SystemSnapshot;
use crate::treemap::algorithm::squarify;
use crate::treemap::color::apply_memory_heatmap;
use crate::treemap::node::{LayoutRect, TreemapItem, TreemapRect};

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct App {
    pub running: bool,
    pub collector: Collector,
    pub snapshot: SystemSnapshot,
    pub layout_rects: Vec<TreemapRect>,
    pub selected_index: usize,
    needs_relayout: bool,
}

impl App {
    pub fn new() -> Self {
        let mut collector = Collector::new();
        let snapshot = collector.refresh();

        App {
            running: true,
            collector,
            snapshot,
            layout_rects: Vec::new(),
            selected_index: 0,
            needs_relayout: true,
        }
    }

    pub fn refresh_data(&mut self) {
        self.snapshot = self.collector.refresh();
        self.needs_relayout = true;
    }

    pub fn compute_layout(&mut self, width: u16, height: u16) {
        if !self.needs_relayout {
            return;
        }

        let mut items: Vec<TreemapItem> = self
            .snapshot
            .process_tree
            .processes
            .values()
            .filter(|p| p.memory_bytes > 0)
            .map(|p| TreemapItem {
                id: p.pid,
                label: p.name.clone(),
                value: p.memory_bytes,
            })
            .collect();

        items.sort_by(|a, b| b.value.cmp(&a.value));

        let bounds = LayoutRect::new(0.0, 0.0, width as f64, height as f64);
        self.layout_rects = squarify(&items, &bounds);
        apply_memory_heatmap(&mut self.layout_rects, self.snapshot.memory_total);

        if self.selected_index >= self.layout_rects.len() && !self.layout_rects.is_empty() {
            self.selected_index = 0;
        }
        self.needs_relayout = false;
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Up => self.navigate(Direction::Up),
            KeyCode::Down => self.navigate(Direction::Down),
            KeyCode::Left => self.navigate(Direction::Left),
            KeyCode::Right => self.navigate(Direction::Right),
            _ => {}
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

    pub fn on_resize(&mut self) {
        self.needs_relayout = true;
    }
}
