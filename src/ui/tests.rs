use std::collections::{HashMap, VecDeque};

use insta::assert_snapshot;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

use crate::app::InputMode;
use crate::system::history::HistoryStore;
use crate::system::process::{ProcessInfo, ProcessState, ProcessTree};
use crate::system::snapshot::SystemSnapshot;
use crate::treemap::node::LayoutRect;
use crate::ui::theme::{
    BorderStyle, ColorMode, ColorSupport, ColoredTreemapRect, HeatOverrides, Theme,
};
use crate::ui::{detail_panel, header, selection_bar, statusbar, treemap_widget};

fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = buf.cell((x, y)).unwrap();
            out.push_str(cell.symbol());
        }
        if y + 1 < area.height {
            out.push('\n');
        }
    }
    out
}

fn render_to_string<F>(width: u16, height: u16, draw: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(draw).unwrap();
    let buf = terminal.backend().buffer();
    buffer_to_string(buf)
}

fn make_process(pid: u32, name: &str, memory: u64, cpu: f32) -> ProcessInfo {
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

fn make_snapshot() -> SystemSnapshot {
    let mut processes = HashMap::new();
    processes.insert(1, make_process(1, "alpha", 200_000_000, 12.5));
    processes.insert(2, make_process(2, "beta", 120_000_000, 7.2));
    SystemSnapshot {
        cpu_usage_percent: 12.5,
        memory_total: 1_024_000_000,
        memory_used: 420_000_000,
        swap_total: 512_000_000,
        swap_used: 64_000_000,
        cpu_per_core: vec![],
        load_average: [0.0; 3],
        process_tree: ProcessTree { processes },
    }
}

fn make_theme() -> Theme {
    let heat = HeatOverrides {
        low: "#2d5a27".to_string(),
        mid: "#b5890a".to_string(),
        high: "#a12e2e".to_string(),
    };
    Theme::from_config("vivid", &heat, ColorSupport::Truecolor)
}

#[test]
fn snapshot_header() {
    let snapshot = make_snapshot();
    let mut cpu_history = VecDeque::new();
    cpu_history.extend([500, 1200, 900, 1500, 2000, 800]);

    let output = render_to_string(80, 3, |frame| {
        header::render(
            frame,
            Rect::new(0, 0, 80, 3),
            &snapshot,
            ColorMode::ByMemory,
            &make_theme(),
            BorderStyle::Rounded,
            &[(1, "alpha".to_string())],
            &cpu_history,
        );
    });

    assert_snapshot!("ui_header", output);
}

#[test]
fn snapshot_statusbar() {
    let output = render_to_string(80, 1, |frame| {
        statusbar::render(
            frame,
            Rect::new(0, 0, 80, 1),
            InputMode::Normal,
            "",
            None,
            &make_theme(),
            true,
        );
    });

    assert_snapshot!("ui_statusbar", output);
}

#[test]
fn snapshot_selection_bar_empty() {
    let output = render_to_string(80, 1, |frame| {
        selection_bar::render(frame, Rect::new(0, 0, 80, 1), None, &make_theme());
    });

    assert_snapshot!("ui_selection_bar_empty", output);
}

#[test]
fn snapshot_selection_bar_with_selection() {
    let output = render_to_string(80, 1, |frame| {
        selection_bar::render(
            frame,
            Rect::new(0, 0, 80, 1),
            Some(selection_bar::SelectionInfo {
                pid: 1,
                name: "Brave Browser Helper".to_string(),
                memory_bytes: 571_400_000,
            }),
            &make_theme(),
        );
    });

    assert_snapshot!("ui_selection_bar_with_selection", output);
}

#[test]
fn snapshot_selection_bar_truncation_right_memory_preserved() {
    let output = render_to_string(24, 1, |frame| {
        selection_bar::render(
            frame,
            Rect::new(0, 0, 24, 1),
            Some(selection_bar::SelectionInfo {
                pid: 99,
                name: "Extremely Long Application Name With Suffix".to_string(),
                memory_bytes: 1_234_567_890,
            }),
            &make_theme(),
        );
    });

    assert!(output.ends_with("1.1 GB"));
    assert_snapshot!("ui_selection_bar_truncation", output);
}

#[test]
fn snapshot_detail_panel() {
    let snapshot = make_snapshot();
    let process = snapshot.process_tree.processes.get(&1).unwrap();
    let mut store = HistoryStore::new(10);
    for i in 0..6 {
        store.record(process.pid, 100_000_000 + i * 10_000_000, i as f32 * 5.0);
    }
    let history = store.get(process.pid);

    let output = render_to_string(40, 16, |frame| {
        detail_panel::render(
            frame,
            Rect::new(0, 0, 40, 16),
            process,
            &make_theme(),
            BorderStyle::Rounded,
            history,
        );
    });

    assert_snapshot!("ui_detail_panel", output);
}

#[test]
fn snapshot_treemap_widget() {
    let rects = vec![
        ColoredTreemapRect {
            rect: LayoutRect::new(0.0, 0.0, 20.0, 6.0),
            pid: 1,
            label: "alpha".to_string(),
            value: 200_000_000,
            color: ratatui::style::Color::Rgb(120, 200, 140),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(20.0, 0.0, 20.0, 6.0),
            pid: 2,
            label: "beta".to_string(),
            value: 120_000_000,
            color: ratatui::style::Color::Rgb(200, 160, 90),
        },
    ];

    let output = render_to_string(40, 6, |frame| {
        treemap_widget::render(
            frame,
            Rect::new(0, 0, 40, 6),
            &rects,
            0,
            6,
            2,
            BorderStyle::Rounded,
            &make_theme(),
        );
    });

    assert_snapshot!("ui_treemap_widget", output);
}

#[test]
fn snapshot_treemap_selected_warm_block() {
    let rects = vec![
        ColoredTreemapRect {
            rect: LayoutRect::new(0.0, 0.0, 24.0, 7.0),
            pid: 1,
            label: "critical".to_string(),
            value: 600_000_000,
            color: ratatui::style::Color::Rgb(249, 115, 22),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(24.0, 0.0, 16.0, 7.0),
            pid: 2,
            label: "normal".to_string(),
            value: 120_000_000,
            color: ratatui::style::Color::Rgb(16, 185, 129),
        },
    ];

    let output = render_to_string(40, 7, |frame| {
        treemap_widget::render(
            frame,
            Rect::new(0, 0, 40, 7),
            &rects,
            0,
            6,
            2,
            BorderStyle::Rounded,
            &make_theme(),
        );
    });

    assert_snapshot!("ui_treemap_selected_warm", output);
}

#[test]
fn snapshot_treemap_other_group_present() {
    let rects = vec![
        ColoredTreemapRect {
            rect: LayoutRect::new(0.0, 0.0, 26.0, 7.0),
            pid: 0,
            label: "Other (349 procs, 1.4 GB)".to_string(),
            value: 1_400_000_000,
            color: ratatui::style::Color::Rgb(49, 50, 68),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(26.0, 0.0, 14.0, 7.0),
            pid: 42,
            label: "brave".to_string(),
            value: 420_000_000,
            color: ratatui::style::Color::Rgb(239, 68, 68),
        },
    ];

    let output = render_to_string(40, 7, |frame| {
        treemap_widget::render(
            frame,
            Rect::new(0, 0, 40, 7),
            &rects,
            1,
            6,
            2,
            BorderStyle::Rounded,
            &make_theme(),
        );
    });

    assert_snapshot!("ui_treemap_other_group", output);
}

#[test]
fn snapshot_treemap_mixed_palette() {
    let rects = vec![
        ColoredTreemapRect {
            rect: LayoutRect::new(0.0, 0.0, 18.0, 8.0),
            pid: 1,
            label: "brave".to_string(),
            value: 400_000_000,
            color: ratatui::style::Color::Rgb(96, 165, 250),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(18.0, 0.0, 16.0, 8.0),
            pid: 2,
            label: "code".to_string(),
            value: 360_000_000,
            color: ratatui::style::Color::Rgb(251, 146, 60),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(34.0, 0.0, 14.0, 4.0),
            pid: 3,
            label: "node".to_string(),
            value: 180_000_000,
            color: ratatui::style::Color::Rgb(45, 212, 191),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(34.0, 4.0, 14.0, 4.0),
            pid: 0,
            label: "other".to_string(),
            value: 140_000_000,
            color: ratatui::style::Color::Rgb(49, 50, 68),
        },
    ];

    let output = render_to_string(48, 8, |frame| {
        treemap_widget::render(
            frame,
            Rect::new(0, 0, 48, 8),
            &rects,
            1,
            6,
            2,
            BorderStyle::Rounded,
            &make_theme(),
        );
    });

    assert_snapshot!("ui_treemap_mixed_palette", output);
}

#[test]
fn snapshot_treemap_flush_tiles() {
    let rects = vec![
        ColoredTreemapRect {
            rect: LayoutRect::new(0.0, 0.0, 16.0, 8.0),
            pid: 11,
            label: "code".to_string(),
            value: 300_000_000,
            color: ratatui::style::Color::Rgb(251, 146, 60),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(16.0, 0.0, 16.0, 8.0),
            pid: 22,
            label: "brave".to_string(),
            value: 280_000_000,
            color: ratatui::style::Color::Rgb(96, 165, 250),
        },
        ColoredTreemapRect {
            rect: LayoutRect::new(32.0, 0.0, 16.0, 8.0),
            pid: 33,
            label: "node".to_string(),
            value: 180_000_000,
            color: ratatui::style::Color::Rgb(45, 212, 191),
        },
    ];

    let output = render_to_string(48, 8, |frame| {
        treemap_widget::render(
            frame,
            Rect::new(0, 0, 48, 8),
            &rects,
            1,
            6,
            2,
            BorderStyle::Rounded,
            &make_theme(),
        );
    });

    assert_snapshot!("ui_treemap_flush_tiles", output);
}
