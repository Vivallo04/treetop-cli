use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use std::hint::black_box;
use treetop::system::process::{ProcessInfo, ProcessState, build_process_tree_from_flat};
use treetop::treemap::algorithm::squarify_sorted;
use treetop::treemap::node::{LayoutRect, TreemapItem};
use treetop::ui::theme::{
    BorderStyle, ColorMode, ColorSupport, HeatOverrides, Theme, colorize_rects,
};
use treetop::ui::treemap_widget;

fn make_items(n: usize) -> Vec<TreemapItem> {
    (0..n)
        .map(|i| TreemapItem {
            pid: i as u32 + 1,
            label: format!("proc_{i}"),
            value: ((n - i) as u64 + 1) * 1024,
        })
        .collect()
}

fn make_processes(n: usize) -> Vec<ProcessInfo> {
    (0..n)
        .map(|i| {
            let pid = i as u32 + 1;
            let ppid = if i == 0 { 0 } else { (i as u32 / 2) + 1 };
            ProcessInfo {
                pid,
                ppid,
                name: format!("proc_{i}"),
                command: format!("proc_{i} --work"),
                memory_bytes: ((n - i) as u64 + 1) * 1024,
                cpu_percent: (i % 100) as f32,
                user_id: Some(format!("u{}", i % 8)),
                group_id: Some(format!("g{}", i % 4)),
                status: ProcessState::Running,
                children: Vec::new(),
                group_name: None,
                priority: None,
                io_stats: None,
            }
        })
        .collect()
}

fn make_theme() -> Theme {
    let heat = HeatOverrides {
        low: "#2d5a27".to_string(),
        mid: "#b5890a".to_string(),
        high: "#a12e2e".to_string(),
    };
    Theme::from_config("vivid", &heat, ColorSupport::Truecolor)
}

fn bench_squarify(c: &mut Criterion) {
    let mut group = c.benchmark_group("squarify_500_1000_2000");
    let bounds = LayoutRect::new(0.0, 0.0, 160.0, 50.0);

    for size in [500usize, 1000, 2000] {
        let items = make_items(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &items, |b, items| {
            b.iter(|| {
                let mut sorted = black_box(items.clone());
                sorted.sort_by(|a, b| b.value.cmp(&a.value));
                let rects = squarify_sorted(black_box(&sorted), black_box(&bounds));
                black_box(rects);
            })
        });
    }

    group.finish();
}

fn bench_layout_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout_pipeline_500_1000_2000");
    let bounds = LayoutRect::new(0.0, 0.0, 160.0, 50.0);

    for size in [500usize, 1000, 2000] {
        let items = make_items(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &items, |b, items| {
            b.iter(|| {
                let mut sorted = black_box(items.clone());
                sorted.sort_by(|a, b| b.value.cmp(&a.value));
                let rects = squarify_sorted(black_box(&sorted), black_box(&bounds));
                black_box(rects);
            })
        });
    }

    group.finish();
}

fn bench_process_tree_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_tree_build_500_1000_2000");

    for size in [500usize, 1000, 2000] {
        let processes = make_processes(size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &processes,
            |b, processes| {
                b.iter(|| {
                    let tree = build_process_tree_from_flat(black_box(processes.clone()));
                    black_box(tree);
                })
            },
        );
    }

    group.finish();
}

fn bench_treemap_widget_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("treemap_widget_render_500_1000_2000");
    let bounds = LayoutRect::new(0.0, 0.0, 160.0, 50.0);
    let theme = make_theme();

    for size in [500usize, 1000, 2000] {
        let items = make_items(size);
        let mut sorted = items.clone();
        sorted.sort_by(|a, b| b.value.cmp(&a.value));

        let base_rects = squarify_sorted(&sorted, &bounds);
        let process_tree = build_process_tree_from_flat(make_processes(size));
        let total_memory: u64 = sorted.iter().map(|i| i.value).sum();
        let colored = colorize_rects(
            &base_rects,
            &process_tree,
            total_memory,
            ColorMode::ByMemory,
            &theme,
            ColorSupport::Truecolor,
        );

        group.bench_with_input(BenchmarkId::from_parameter(size), &colored, |b, colored| {
            b.iter(|| {
                let backend = TestBackend::new(160, 50);
                let mut terminal = Terminal::new(backend).expect("bench terminal init failed");
                terminal
                    .draw(|frame| {
                        treemap_widget::render(
                            frame,
                            Rect::new(0, 0, 160, 50),
                            black_box(colored),
                            0,
                            6,
                            2,
                            BorderStyle::Rounded,
                            &theme,
                        );
                    })
                    .expect("bench draw failed");
                black_box(terminal.backend());
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_squarify,
    bench_layout_pipeline,
    bench_process_tree_build,
    bench_treemap_widget_render
);
criterion_main!(benches);
