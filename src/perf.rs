use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{Result, eyre};
use serde::Serialize;
use serde_json::Value;

const TRACKED_SPANS: [&str; 3] = [
    "collector.refresh",
    "app.compute_layout",
    "ui.treemap_widget.render",
];

const BENCH_GROUPS: [&str; 4] = [
    "squarify_500_1000_2000",
    "layout_pipeline_500_1000_2000",
    "process_tree_build_500_1000_2000",
    "treemap_widget_render_500_1000_2000",
];

#[cfg(feature = "perf-tracing")]
pub fn init_tracing_json(output_path: &Path) -> Result<()> {
    use tracing_subscriber::fmt::format::FmtSpan;

    ensure_parent_dir(output_path)?;
    let file = File::create(output_path)?;
    let make_writer = move || {
        file.try_clone()
            .expect("failed to clone perf tracing output file")
    };

    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .json()
        .with_span_events(FmtSpan::CLOSE)
        .with_max_level(tracing::Level::DEBUG)
        .with_writer(make_writer)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|e| eyre!("failed to set tracing subscriber: {e}"))?;
    Ok(())
}

pub fn write_baseline_artifacts(
    span_log_path: &Path,
    iterations: usize,
    width: u16,
    height: u16,
    process_counts: &[usize],
) -> Result<()> {
    let span_stats = parse_span_stats(span_log_path)?;
    let criterion = parse_criterion_baselines()?;
    let process_count_stats = summarize_process_counts(process_counts)?;

    let baseline = PerfBaseline {
        generated_at_unix_s: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| eyre!("system clock is before UNIX_EPOCH: {e}"))?
            .as_secs(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        perf_capture: PerfCaptureBaseline {
            iterations,
            width,
            height,
            span_log_path: span_log_path.display().to_string(),
            process_counts: process_count_stats,
            spans: span_stats,
        },
        criterion,
    };

    let docs_dir = Path::new("docs");
    fs::create_dir_all(docs_dir)?;

    let json_path = docs_dir.join("perf_baseline.json");
    let markdown_path = docs_dir.join("PERF_BASELINE.md");

    let json = serde_json::to_string_pretty(&baseline)?;
    fs::write(&json_path, json)?;

    let markdown = render_markdown(&baseline);
    fs::write(&markdown_path, markdown)?;

    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn parse_span_stats(path: &Path) -> Result<BTreeMap<String, SpanStats>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut samples: HashMap<String, Vec<f64>> = HashMap::new();
    for &name in &TRACKED_SPANS {
        samples.insert(name.to_string(), Vec::new());
    }

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let Some(span_name) = extract_span_name(&value) else {
            continue;
        };
        if !samples.contains_key(span_name) {
            continue;
        }

        let Some(duration_raw) = extract_busy_duration(&value) else {
            continue;
        };
        let Some(us) = parse_duration_to_us(duration_raw) else {
            continue;
        };

        if let Some(vec) = samples.get_mut(span_name) {
            vec.push(us);
        }
    }

    let mut out = BTreeMap::new();
    for &name in &TRACKED_SPANS {
        let mut values = samples.remove(name).unwrap_or_default();
        let stats = summarize_samples(&mut values);
        out.insert(name.to_string(), stats);
    }
    Ok(out)
}

fn parse_criterion_baselines() -> Result<BTreeMap<String, BTreeMap<String, f64>>> {
    let root = Path::new("target").join("criterion");
    let mut groups: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();

    for group in BENCH_GROUPS {
        let mut sizes = BTreeMap::new();
        for size in [500usize, 1000, 2000] {
            let path = root
                .join(group)
                .join(size.to_string())
                .join("new")
                .join("estimates.json");
            if !path.exists() {
                continue;
            }
            let contents = fs::read_to_string(&path)?;
            let value: Value = serde_json::from_str(&contents)?;
            if let Some(ns) = value
                .get("median")
                .and_then(|v| v.get("point_estimate"))
                .and_then(Value::as_f64)
            {
                sizes.insert(size.to_string(), round_2(ns / 1000.0));
            }
        }

        if !sizes.is_empty() {
            groups.insert(group.to_string(), sizes);
        }
    }

    Ok(groups)
}

fn extract_span_name(value: &Value) -> Option<&str> {
    value
        .get("span")
        .and_then(|span| span.get("name"))
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("spans")
                .and_then(Value::as_array)
                .and_then(|arr| arr.last())
                .and_then(|span| span.get("name"))
                .and_then(Value::as_str)
        })
}

fn extract_busy_duration(value: &Value) -> Option<&str> {
    value
        .get("fields")
        .and_then(|f| f.get("time.busy"))
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("fields")
                .and_then(|f| f.get("busy"))
                .and_then(Value::as_str)
        })
}

fn parse_duration_to_us(raw: &str) -> Option<f64> {
    let s = raw.trim();
    if let Some(v) = s.strip_suffix("ns") {
        return v.trim().parse::<f64>().ok().map(|n| n / 1000.0);
    }
    if let Some(v) = s.strip_suffix("µs").or_else(|| s.strip_suffix("μs")) {
        return v.trim().parse::<f64>().ok();
    }
    if let Some(v) = s.strip_suffix("us") {
        return v.trim().parse::<f64>().ok();
    }
    if let Some(v) = s.strip_suffix("ms") {
        return v.trim().parse::<f64>().ok().map(|ms| ms * 1000.0);
    }
    if let Some(v) = s.strip_suffix('s') {
        return v.trim().parse::<f64>().ok().map(|secs| secs * 1_000_000.0);
    }
    None
}

fn summarize_samples(values: &mut [f64]) -> SpanStats {
    if values.is_empty() {
        return SpanStats {
            count: 0,
            p50_us: 0.0,
            p95_us: 0.0,
            max_us: 0.0,
        };
    }
    values.sort_by(|a, b| a.total_cmp(b));

    let last = values.len() - 1;
    let p50_idx = ((last as f64) * 0.50).round() as usize;
    let p95_idx = ((last as f64) * 0.95).round() as usize;

    SpanStats {
        count: values.len(),
        p50_us: round_2(values[p50_idx]),
        p95_us: round_2(values[p95_idx]),
        max_us: round_2(values[last]),
    }
}

fn summarize_process_counts(process_counts: &[usize]) -> Result<ProcessCountStats> {
    if process_counts.is_empty() {
        return Err(eyre!("no process counts captured during perf run"));
    }
    let mut values = process_counts.to_vec();
    values.sort_unstable();
    let min = *values.first().unwrap_or(&0);
    let max = *values.last().unwrap_or(&0);
    let p50 = values[(values.len() - 1) / 2];
    Ok(ProcessCountStats { min, p50, max })
}

fn round_2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn render_markdown(baseline: &PerfBaseline) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Performance Baseline");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Generated (unix): `{}`",
        baseline.generated_at_unix_s
    );
    let _ = writeln!(out, "- Platform: `{}` / `{}`", baseline.os, baseline.arch);
    let _ = writeln!(
        out,
        "- Perf capture: `{}` iterations, `{}`x`{}` terminal",
        baseline.perf_capture.iterations, baseline.perf_capture.width, baseline.perf_capture.height
    );
    let _ = writeln!(
        out,
        "- Observed process counts: min `{}`, p50 `{}`, max `{}`",
        baseline.perf_capture.process_counts.min,
        baseline.perf_capture.process_counts.p50,
        baseline.perf_capture.process_counts.max
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## Span Timings (`us`)");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Span | Count | p50 | p95 | max |");
    let _ = writeln!(out, "| --- | ---: | ---: | ---: | ---: |");
    for (name, stats) in &baseline.perf_capture.spans {
        let _ = writeln!(
            out,
            "| `{}` | {} | {:.2} | {:.2} | {:.2} |",
            name, stats.count, stats.p50_us, stats.p95_us, stats.max_us
        );
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Criterion Benchmarks (`us` median point estimate)");
    let _ = writeln!(out);

    if baseline.criterion.is_empty() {
        let _ = writeln!(
            out,
            "_No criterion results found under `target/criterion`._"
        );
    } else {
        let _ = writeln!(out, "| Benchmark Group | 500 | 1000 | 2000 |");
        let _ = writeln!(out, "| --- | ---: | ---: | ---: |");
        for (group, sizes) in &baseline.criterion {
            let v500 = sizes.get("500").copied().unwrap_or(0.0);
            let v1000 = sizes.get("1000").copied().unwrap_or(0.0);
            let v2000 = sizes.get("2000").copied().unwrap_or(0.0);
            let _ = writeln!(
                out,
                "| `{}` | {:.2} | {:.2} | {:.2} |",
                group, v500, v1000, v2000
            );
        }
    }

    out
}

#[derive(Debug, Serialize)]
struct PerfBaseline {
    generated_at_unix_s: u64,
    os: String,
    arch: String,
    perf_capture: PerfCaptureBaseline,
    criterion: BTreeMap<String, BTreeMap<String, f64>>,
}

#[derive(Debug, Serialize)]
struct PerfCaptureBaseline {
    iterations: usize,
    width: u16,
    height: u16,
    span_log_path: String,
    process_counts: ProcessCountStats,
    spans: BTreeMap<String, SpanStats>,
}

#[derive(Debug, Serialize)]
struct ProcessCountStats {
    min: usize,
    p50: usize,
    max: usize,
}

#[derive(Debug, Serialize)]
struct SpanStats {
    count: usize,
    p50_us: f64,
    p95_us: f64,
    max_us: f64,
}

#[cfg(test)]
mod tests {
    use super::parse_duration_to_us;

    #[test]
    fn duration_parsing_supported_units() {
        assert_eq!(parse_duration_to_us("100ns"), Some(0.1));
        assert_eq!(parse_duration_to_us("10us"), Some(10.0));
        assert_eq!(parse_duration_to_us("10µs"), Some(10.0));
        assert_eq!(parse_duration_to_us("2.5ms"), Some(2500.0));
        assert_eq!(parse_duration_to_us("1s"), Some(1_000_000.0));
    }
}
