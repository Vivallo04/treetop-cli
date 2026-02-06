# Performance Baseline

- Generated (unix): `1770391271`
- Platform: `macos` / `aarch64`
- Perf capture: `120` iterations, `160`x`50` terminal
- Observed process counts: min `573`, p50 `573`, max `573`

## Span Timings (`us`)

| Span | Count | p50 | p95 | max |
| --- | ---: | ---: | ---: | ---: |
| `app.compute_layout` | 120 | 207.00 | 308.00 | 479.00 |
| `collector.refresh` | 121 | 9370.00 | 10100.00 | 16200.00 |
| `ui.treemap_widget.render` | 120 | 1190.00 | 1310.00 | 1650.00 |

## Criterion Benchmarks (`us` median point estimate)

| Benchmark Group | 500 | 1000 | 2000 |
| --- | ---: | ---: | ---: |
| `layout_pipeline_500_1000_2000` | 42.60 | 93.69 | 210.80 |
| `process_tree_build_500_1000_2000` | 121.58 | 245.03 | 496.07 |
| `squarify_500_1000_2000` | 42.41 | 94.44 | 210.00 |
| `treemap_widget_render_500_1000_2000` | 391.51 | 354.21 | 336.11 |
