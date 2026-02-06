# treetop — Cross-Platform TUI Treemap Process Monitor

## Technical Architecture Plan

**Author:** Senior Systems Engineer  
**Target Platforms:** Linux, macOS, Windows  
**Language:** Rust  
**License:** MIT / Apache-2.0 (dual-license, standard for Rust ecosystem)

---

## 1. Executive Summary

treetop is a terminal-based system monitor that visualizes memory consumption using an interactive treemap layout. Unlike traditional list-based monitors (`htop`, `top`, `btm`), treetop renders processes as proportionally-sized rectangles — giving users an instant spatial understanding of what's consuming their RAM.

The tool must run on Linux, macOS, and Windows with zero runtime dependencies, minimal resource footprint, and a consistent user experience across platforms.

---

## 2. Platform Abstraction Strategy

### 2.1 The Core Problem

Each OS exposes process and system metrics through fundamentally different interfaces:

| Data Source | Linux | macOS | Windows |
|---|---|---|---|
| Process list | `/proc/<pid>/stat`, `/proc/<pid>/status` | `libproc` / `sysctl` | `NtQuerySystemInformation`, `Toolhelp32` |
| Memory info | `/proc/meminfo` | `host_statistics64` | `GlobalMemoryStatusEx` |
| CPU usage | `/proc/stat`, `/proc/<pid>/stat` | `processor_info` | `GetSystemTimes`, `NtQuerySystemInformation` |
| Process tree | `/proc/<pid>/status` (PPid) | `libproc` (ppid) | `PROCESSENTRY32.th32ParentProcessID` |
| Per-process I/O | `/proc/<pid>/io` | Partial via `rusage` | `GetProcessIoCounters` |
| GPU usage | NVML / `/sys/class/drm` | IOKit | NVML / D3DKMT |

### 2.2 Abstraction Approach

We do **not** reimplement this from scratch. The `sysinfo` crate already abstracts all of this with a unified API. We build a thin domain layer on top that maps raw `sysinfo` data into our internal model:

```
┌─────────────────────────────────────────────────┐
│                   UI Layer                       │
│          (ratatui widgets, treemap)              │
├─────────────────────────────────────────────────┤
│                Domain Layer                      │
│     ProcessTree, SystemSnapshot, Metrics         │
├─────────────────────────────────────────────────┤
│              Platform Layer                      │
│   sysinfo + targeted #[cfg] extensions           │
├──────────┬──────────────┬───────────────────────┤
│  Linux   │    macOS     │      Windows           │
│ /proc/*  │  libproc     │  Win32 API             │
└──────────┴──────────────┴───────────────────────┘
```

The domain layer normalizes everything into platform-agnostic structs. The UI layer never touches platform-specific code.

### 2.3 Platform-Specific Extensions via `#[cfg]`

For data that `sysinfo` doesn't cover (or covers poorly), we add targeted extensions:

```rust
// src/system/platform/mod.rs
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub use self::platform_impl::*;

#[cfg(target_os = "linux")]
use linux as platform_impl;
#[cfg(target_os = "macos")]
use macos as platform_impl;
#[cfg(target_os = "windows")]
use windows as platform_impl;
```

Each platform module implements the same trait:

```rust
pub trait PlatformExtensions {
    /// cgroup or container name (Linux), App bundle name (macOS), Service name (Windows)
    fn process_group_name(pid: u32) -> Option<String>;

    /// OOM score or priority
    fn process_priority(pid: u32) -> Option<i32>;

    /// IO bytes (if sysinfo's coverage is insufficient)
    fn process_io(pid: u32) -> Option<IoStats>;
}
```

---

## 3. Project Architecture

### 3.1 Directory Structure

```
treetop/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .github/
│   └── workflows/
│       ├── ci.yml                # Test on Linux, macOS, Windows
│       └── release.yml           # Cross-compile + publish binaries
├── assets/
│   ├── demo.gif                  # Terminal recording for README
│   └── screenshot.png
├── src/
│   ├── main.rs                   # Entry point, arg parsing, panic handler
│   ├── app.rs                    # App state machine, event dispatch
│   ├── config.rs                 # Configuration (file + CLI overrides)
│   ├── event.rs                  # Event loop (keyboard, mouse, tick, resize)
│   │
│   ├── system/                   # Data collection layer
│   │   ├── mod.rs
│   │   ├── snapshot.rs           # SystemSnapshot: point-in-time system state
│   │   ├── process.rs            # ProcessInfo struct, tree building
│   │   ├── collector.rs          # Polling loop, diff calculation
│   │   └── platform/             # Platform-specific extensions
│   │       ├── mod.rs
│   │       ├── linux.rs          # cgroup names, OOM scores
│   │       ├── macos.rs          # App bundle resolution
│   │       └── windows.rs        # Service names, UWP app names
│   │
│   ├── treemap/                  # Treemap engine (pure algorithm, no TUI dependency)
│   │   ├── mod.rs
│   │   ├── algorithm.rs          # Squarified treemap layout algorithm
│   │   ├── node.rs               # TreemapNode, TreemapRect
│   │   └── color.rs              # Color mapping strategies
│   │
│   ├── ui/                       # Ratatui rendering
│   │   ├── mod.rs
│   │   ├── treemap_widget.rs     # Custom Widget impl for treemap
│   │   ├── header.rs             # Top bar: CPU, RAM, swap summary
│   │   ├── statusbar.rs          # Bottom bar: keybinds, filter status
│   │   ├── detail_panel.rs       # Side panel: selected process details
│   │   ├── sparkline.rs          # Mini history charts
│   │   ├── help.rs               # Help overlay
│   │   └── theme.rs              # Color schemes (dark, light, colorblind)
│   │
│   └── action.rs                 # Action enum (Kill, Filter, Sort, Zoom, etc.)
│
├── tests/
│   ├── treemap_layout_tests.rs   # Property tests for squarified algorithm
│   └── snapshot_tests.rs         # Mock system data → verify tree building
│
└── benches/
    └── treemap_bench.rs          # Layout perf with 500+ processes
```

### 3.2 Core Data Flow

```
                    ┌──────────────┐
                    │  Tick Timer  │  (every 1-2s, configurable)
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Collector   │  sysinfo::System::refresh_all()
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Snapshot    │  Normalized ProcessInfo Vec
                    │  Builder     │  + build parent-child tree
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  App State   │  Apply filters, sort, zoom context
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Treemap     │  squarify() → Vec<TreemapRect>
                    │  Layout      │  (pure function, testable)
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │  Ratatui     │  Render rectangles to Buffer
                    │  Widget      │  + labels + borders + colors
                    └──────────────┘
```

### 3.3 Key Data Structures

```rust
/// Normalized process info, platform-agnostic
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub command: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub user: String,
    pub state: ProcessState,
    pub group: Option<String>,    // cgroup (Linux), bundle (macOS), service (Windows)
    pub children: Vec<u32>,       // child PIDs
}

/// The full tree of processes, ready for treemap input
#[derive(Clone, Debug)]
pub struct ProcessTree {
    pub processes: HashMap<u32, ProcessInfo>,
    pub roots: Vec<u32>,          // top-level processes (ppid=0 or ppid=1)
    pub total_memory: u64,
}

/// Output of the squarified treemap algorithm
#[derive(Clone, Debug)]
pub struct TreemapRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub pid: u32,
    pub label: String,
    pub value: u64,               // memory bytes
    pub depth: u8,                // nesting level (for zoom)
    pub color: Color,
}

/// Point-in-time system state
pub struct SystemSnapshot {
    pub timestamp: Instant,
    pub cpu_usage: Vec<f32>,      // per-core
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub process_tree: ProcessTree,
    pub load_average: [f64; 3],   // Linux/macOS only
}
```

---

## 4. Crate Dependencies

### 4.1 Core Dependencies

| Crate | Version | Purpose | Notes |
|---|---|---|---|
| `ratatui` | latest | TUI framework | Immediate-mode rendering, custom widgets |
| `crossterm` | latest | Terminal backend | Cross-platform (Linux, macOS, Windows). Replaces termion (Unix-only) |
| `sysinfo` | latest | Process/CPU/memory data | Supports Linux, macOS, Windows, FreeBSD |
| `tokio` | latest (rt, macros, time) | Async runtime | Event loop, tick timer, signal handling |
| `clap` | latest (derive) | CLI argument parsing | Subcommands, env var support |
| `serde` + `toml` | latest | Configuration | `~/.config/treetop/config.toml` |
| `dirs` | latest | Platform config paths | XDG on Linux, `~/Library` on macOS, `%APPDATA%` on Windows |
| `log` + `tracing` | latest | Structured logging | Debug file logging, no stdout pollution |

### 4.2 Optional / Feature-Gated

| Crate | Feature | Purpose |
|---|---|---|
| `unicode-width` | default | Correct CJK/emoji character widths in labels |
| `better-panic` | default | Pretty panic output during development |
| `color-eyre` | default | Colorful error reports |
| `signal-hook` | unix | SIGWINCH (resize), SIGTERM handling |
| `ctrlc` | default | Graceful Ctrl-C on all platforms |

### 4.3 Dev / Test

| Crate | Purpose |
|---|---|
| `criterion` | Benchmarks for treemap layout with large process counts |
| `proptest` | Property-based testing for layout algorithm invariants |
| `insta` | Snapshot testing for rendered terminal output |
| `mockall` | Mock sysinfo traits for deterministic tests |

### 4.4 Why NOT These

| Crate | Reason to skip |
|---|---|
| `termion` | Unix-only, no Windows support |
| `ncurses` / `pancurses` | C dependency, harder to cross-compile, less idiomatic |
| `tui-rs` | Deprecated — `ratatui` is the maintained fork |
| `procfs` | Linux-only; `sysinfo` already covers this cross-platform |

---

## 5. The Squarified Treemap Algorithm

### 5.1 Algorithm Overview

Standard reference: *Squarified Treemaps* (Bruls, Huizing & van Wijk, 2000).

Goal: lay out rectangles proportional to their value (RAM) while keeping aspect ratios as close to 1:1 as possible.

```
Input:  [Chrome: 2GB, Firefox: 1GB, VSCode: 800MB, Slack: 500MB, ...]
Output: ┌─────────────────────┬────────────┐
        │                     │  Firefox   │
        │     Chrome (2GB)    │   (1GB)    │
        │                     ├──────┬─────┤
        │                     │VSCode│Slack│
        └─────────────────────┴──────┴─────┘
```

### 5.2 Implementation Pseudocode

```rust
pub fn squarify(items: &[TreemapItem], bounds: Rect) -> Vec<TreemapRect> {
    // 1. Sort items descending by value
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.value.cmp(&a.value));

    let total: f64 = sorted.iter().map(|i| i.value as f64).sum();
    let mut results = Vec::new();
    let mut remaining = bounds;

    let mut row: Vec<&TreemapItem> = Vec::new();
    let mut row_area = 0.0;

    for item in &sorted {
        let item_area = (item.value as f64 / total) * area(bounds);

        // Try adding this item to current row
        row.push(item);
        row_area += item_area;

        let worst_current = worst_aspect_ratio(&row, row_area, shorter_side(remaining));

        // Check if adding this item made the row worse
        row.pop();
        let row_without = if row.is_empty() {
            f64::MAX
        } else {
            let prev_area = row_area - item_area;
            worst_aspect_ratio(&row, prev_area, shorter_side(remaining))
        };

        if row.is_empty() || worst_current <= row_without {
            // Adding improves (or doesn't worsen) — keep it
            row.push(item);
            row_area += item_area; // (already added above, adjust logic)
        } else {
            // Finalize current row, layout its items
            layout_row(&row, row_area, &mut remaining, &mut results);
            row.clear();
            row = vec![item];
            row_area = item_area;
        }
    }

    // Layout final row
    if !row.is_empty() {
        layout_row(&row, row_area, &mut remaining, &mut results);
    }

    results
}

fn worst_aspect_ratio(row: &[&TreemapItem], row_area: f64, side: f64) -> f64 {
    // For each item in the row, compute its aspect ratio
    // Return the worst (max) one
    // aspect_ratio = max(w/h, h/w) — we want this close to 1.0
}
```

### 5.3 Hierarchical / Nested Treemaps

For the zoom feature (press Enter to drill into a process group):

```rust
pub struct TreemapNode {
    pub item: TreemapItem,
    pub children: Vec<TreemapNode>,
}

/// Recursive treemap: parent's rectangle becomes the bounds for children
pub fn squarify_recursive(node: &TreemapNode, bounds: Rect, depth: u8) -> Vec<TreemapRect> {
    if node.children.is_empty() {
        return vec![TreemapRect { bounds, depth, ..from(node) }];
    }

    let child_items: Vec<TreemapItem> = node.children.iter().map(|c| c.item.clone()).collect();
    let child_rects = squarify(&child_items, shrink(bounds, 1)); // 1-cell border

    let mut results = vec![TreemapRect { bounds, depth, is_group: true, ..from(node) }];
    for (child, rect) in node.children.iter().zip(child_rects) {
        results.extend(squarify_recursive(child, rect.bounds, depth + 1));
    }
    results
}
```

### 5.4 Algorithm Invariants (for property tests)

These must always hold:

1. **Area conservation**: sum of all output rect areas == input bounds area (within float tolerance)
2. **No overlap**: no two output rects share interior points
3. **Containment**: every output rect is fully inside input bounds
4. **Proportionality**: rect areas are proportional to input values
5. **Non-degenerate**: no rect has zero width or height (unless value is 0)

---

## 6. Rendering: Custom Ratatui Widget

### 6.1 Widget Implementation

```rust
pub struct TreemapWidget<'a> {
    rects: &'a [TreemapRect],
    selected: Option<u32>,  // highlighted PID
    color_mode: ColorMode,
}

impl<'a> Widget for TreemapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for rect in self.rects {
            let term_rect = to_terminal_rect(rect, area);

            // 1. Fill background
            fill_rect(buf, term_rect, rect.color);

            // 2. Draw border (single-line box drawing chars)
            draw_border(buf, term_rect, if is_selected(rect) {
                Color::Yellow
            } else {
                Color::DarkGray
            });

            // 3. Render label (truncated to fit)
            let label = truncate_label(&rect.label, term_rect.width as usize - 2);
            if term_rect.width >= 4 && term_rect.height >= 1 {
                buf.set_string(
                    term_rect.x + 1,
                    term_rect.y,
                    &label,
                    Style::default().fg(contrast_color(rect.color)).bold(),
                );
            }

            // 4. Render value (e.g. "1.2 GB") if space allows
            if term_rect.height >= 2 && term_rect.width >= 6 {
                let value_str = format_bytes(rect.value);
                buf.set_string(
                    term_rect.x + 1,
                    term_rect.y + 1,
                    &value_str,
                    Style::default().fg(contrast_color(rect.color)),
                );
            }
        }
    }
}
```

### 6.2 Color Strategies

```rust
pub enum ColorMode {
    ByMemory,     // Heat map: green → yellow → red based on RAM %
    ByCpu,        // Heat map: green → yellow → red based on CPU %
    ByUser,       // Distinct color per user/owner
    ByGroup,      // Distinct color per process group
    Monochrome,   // Grayscale (accessibility)
}

fn color_by_memory(memory_bytes: u64, total_memory: u64) -> Color {
    let pct = memory_bytes as f64 / total_memory as f64;
    match pct {
        p if p > 0.15 => Color::Red,
        p if p > 0.08 => Color::LightRed,
        p if p > 0.04 => Color::Yellow,
        p if p > 0.02 => Color::Green,
        _ => Color::DarkGray,
    }
}
```

---

## 7. Event Loop Architecture

```rust
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,                        // Refresh system data
    Resize(u16, u16),
}

pub enum Action {
    Quit,
    Kill(u32),                   // Send signal to PID
    Select(Direction),           // Navigate treemap
    Zoom(u32),                   // Drill into process group
    ZoomOut,                     // Back to parent
    Filter(String),              // Filter by name
    CycleColorMode,
    CycleSortMode,               // RAM, CPU, PID, name
    ToggleDetailPanel,
    ToggleHelp,
    Refresh,                     // Force immediate refresh
}

/// Main loop (simplified)
async fn run(terminal: &mut Terminal<impl Backend>) -> Result<()> {
    let mut app = App::new();
    let mut event_stream = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_secs(2));

    loop {
        // Draw
        terminal.draw(|frame| app.render(frame))?;

        // Wait for next event
        tokio::select! {
            Some(event) = event_stream.next() => {
                if let Some(action) = app.handle_event(event?) {
                    if action == Action::Quit { break; }
                    app.dispatch(action);
                }
            }
            _ = tick.tick() => {
                app.refresh_system_data();
            }
        }
    }
    Ok(())
}
```

---

## 8. Cross-Platform Concerns

### 8.1 Terminal Compatibility Matrix

| Feature | Linux | macOS Terminal | macOS iTerm2 | Windows Terminal | CMD/PowerShell |
|---|---|---|---|---|---|
| 256 colors | ✅ | ✅ | ✅ | ✅ | ⚠️ (limited) |
| True color (24-bit) | ✅ | ❌ | ✅ | ✅ | ❌ |
| Mouse events | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| Unicode box drawing | ✅ | ✅ | ✅ | ✅ | ✅ |
| Emoji | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| Alternate screen | ✅ | ✅ | ✅ | ✅ | ✅ |
| Bracketed paste | ✅ | ✅ | ✅ | ✅ | ❌ |

**Strategy**: Default to 256-color mode. Detect true-color support via `COLORTERM=truecolor` env var. Provide `--color=256|truecolor|mono` flag.

### 8.2 Process Signals

```rust
// Sending signals to processes
#[cfg(unix)]
fn kill_process(pid: u32, signal: Signal) -> Result<()> {
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        signal,
    )?;
    Ok(())
}

#[cfg(windows)]
fn kill_process(pid: u32, _signal: Signal) -> Result<()> {
    // Windows doesn't have Unix signals.
    // Use TerminateProcess for hard kill, or
    // GenerateConsoleCtrlEvent for Ctrl+C equivalent.
    use windows::Win32::System::Threading::*;
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid)?;
        TerminateProcess(handle, 1)?;
        CloseHandle(handle)?;
    }
    Ok(())
}
```

### 8.3 Configuration Paths

```rust
fn config_path() -> PathBuf {
    // Linux:   ~/.config/treetop/config.toml  (XDG_CONFIG_HOME)
    // macOS:   ~/Library/Application Support/treetop/config.toml
    // Windows: %APPDATA%\treetop\config.toml
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("treetop")
        .join("config.toml")
}
```

### 8.4 Permissions

| Platform | Issue | Solution |
|---|---|---|
| Linux | Cannot read `/proc/<pid>` for other users | Run with `sudo`, or accept partial data. Document in README. |
| macOS | App Sandbox blocks process inspection | No sandbox (CLI tool). Full Disk Access not needed for process info. |
| Windows | Cannot query elevated processes | Request `SeDebugPrivilege` if running as admin. Degrade gracefully. |
| All | Kill requires ownership or root/admin | Check permissions before attempting, show clear error. |

---

## 9. Configuration

```toml
# ~/.config/treetop/config.toml

[general]
refresh_rate_ms = 2000       # How often to poll system data
default_color_mode = "memory" # memory | cpu | user | group | mono
default_sort = "memory"       # memory | cpu | pid | name
show_detail_panel = true
show_kernel_threads = false   # Linux: hide [kworker], [migration], etc.

[treemap]
min_rect_width = 4           # Minimum terminal columns for a visible rect
min_rect_height = 2          # Minimum terminal rows
group_threshold = 0.01       # Group processes using < 1% RAM into "Other"
border_style = "thin"        # thin | thick | none

[colors]
theme = "dark"               # dark | light | colorblind
heat_low = "#2d5a27"         # Low memory usage
heat_mid = "#b5890a"         # Medium
heat_high = "#a12e2e"        # High

[keybinds]
quit = "q"
kill = "k"
force_kill = "K"
filter = "/"
zoom_in = "Enter"
zoom_out = "Escape"
cycle_color = "c"
toggle_detail = "d"
help = "?"
```

---

## 10. CI/CD and Release Pipeline

### 10.1 GitHub Actions: CI

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ matrix.rust }}
          components: clippy, rustfmt
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo bench --no-run  # Compile benches, don't run

  # Ensure it compiles for additional targets
  cross-check:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - x86_64-unknown-linux-musl
          - aarch64-unknown-linux-gnu
          - x86_64-apple-darwin
          - aarch64-apple-darwin
          - x86_64-pc-windows-msvc
    steps:
      - uses: actions/checkout@v4
      - uses: dtolney/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo check --target ${{ matrix.target }}
```

### 10.2 Release Binaries

```yaml
# .github/workflows/release.yml (on tag push)
# Build matrix:
#   - x86_64-unknown-linux-gnu      (Linux x86_64, dynamically linked)
#   - x86_64-unknown-linux-musl     (Linux x86_64, static binary — alpine/docker)
#   - aarch64-unknown-linux-gnu     (Linux ARM64 — Raspberry Pi, AWS Graviton)
#   - x86_64-apple-darwin           (macOS Intel)
#   - aarch64-apple-darwin          (macOS Apple Silicon)
#   - x86_64-pc-windows-msvc        (Windows x86_64)
```

### 10.3 Distribution Channels

| Channel | Method |
|---|---|
| **Cargo** | `cargo install treetop` |
| **Homebrew** | Tap with formula, or submit to homebrew-core |
| **AUR** | Arch Linux package |
| **Scoop / WinGet** | Windows package managers |
| **GitHub Releases** | Pre-built binaries + checksums |
| **Nix** | Flake for NixOS |

---

## 11. Performance Budget

The tool monitors resources — it must not consume them.

| Metric | Target | How |
|---|---|---|
| Binary size | < 5 MB | LTO, strip symbols, `opt-level = "z"` for release |
| Idle CPU | < 0.5% | Only poll on tick interval, no busy loops |
| RAM usage | < 15 MB | No process data cloning, reuse buffers |
| Startup time | < 100ms | No lazy initialization of large data structures |
| Render time | < 16ms per frame | Pre-compute treemap layout, only re-layout on data change |
| Treemap layout | < 5ms for 1000 processes | Benchmark with criterion, optimize hot path |

```toml
# Cargo.toml — Release profile
[profile.release]
lto = "thin"
strip = true
codegen-units = 1
opt-level = 2
```

---

## 12. Testing Strategy

### 12.1 Unit Tests — Treemap Algorithm

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// All output rects must fit within bounds
        #[test]
        fn rects_within_bounds(
            values in prop::collection::vec(1u64..10000, 1..50),
        ) {
            let bounds = Rect { x: 0.0, y: 0.0, width: 120.0, height: 40.0 };
            let items: Vec<TreemapItem> = values.iter().enumerate()
                .map(|(i, &v)| TreemapItem { id: i as u32, value: v, label: format!("p{i}") })
                .collect();
            let rects = squarify(&items, bounds);

            for r in &rects {
                assert!(r.x >= bounds.x);
                assert!(r.y >= bounds.y);
                assert!(r.x + r.width <= bounds.x + bounds.width + 0.001);
                assert!(r.y + r.height <= bounds.y + bounds.height + 0.001);
            }
        }

        /// Total area of output rects ≈ bounds area
        #[test]
        fn area_conservation(
            values in prop::collection::vec(1u64..10000, 1..50),
        ) {
            let bounds = Rect { x: 0.0, y: 0.0, width: 120.0, height: 40.0 };
            let items = /* ... */;
            let rects = squarify(&items, bounds);
            let total_area: f64 = rects.iter().map(|r| r.width * r.height).sum();
            let bounds_area = bounds.width * bounds.height;
            assert!((total_area - bounds_area).abs() < 1.0);
        }
    }
}
```

### 12.2 Integration Tests — Snapshot Testing

Use `insta` to capture rendered terminal output and detect visual regressions:

```rust
#[test]
fn test_treemap_render_snapshot() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let app = App::with_mock_data(/* known process list */);

    terminal.draw(|f| app.render(f)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    insta::assert_snapshot!(buffer_to_string(&buffer));
}
```

### 12.3 Platform Tests

Run on all 3 platforms in CI. Use `sysinfo` mock for deterministic tests, real `sysinfo` for smoke tests.

---

## 13. Roadmap

### Phase 1 — MVP (v0.1)
- [x] Basic squarified treemap rendering
- [x] Live process data via `sysinfo`
- [x] Keyboard navigation (arrow keys, q to quit)
- [x] Color by memory usage
- [x] Header bar with system summary
- [x] Linux + macOS + Windows builds in CI

### Phase 2 — Usable (v0.2)
- [ ] Process filter (`/` to search)
- [ ] Kill process (`k` / `K`)
- [ ] Detail panel for selected process
- [ ] Config file support
- [ ] Multiple color modes
- [ ] Mouse click to select

### Phase 3 — Polished (v0.3)
- [ ] Zoom into process groups (Enter/Escape)
- [ ] Animated transitions between layouts
- [ ] Sparkline history per process
- [ ] Process tree hierarchy (nested treemaps)
- [ ] Theme support (dark, light, colorblind)

### Phase 4 — Ecosystem (v1.0)
- [ ] Homebrew formula, AUR package, Scoop manifest
- [ ] Plugin system for custom data sources
- [ ] GPU monitoring (NVIDIA via NVML)
- [ ] Container-aware (Docker, Podman) process grouping
- [ ] JSON export mode for scripting
- [ ] Remote monitoring via SSH piping

---

## 14. Prior Art and Differentiation

| Tool | Language | What it does | How treetop differs |
|---|---|---|---|
| htop | C | List-based process monitor | Treemap gives spatial RAM overview |
| bottom (btm) | Rust | Graphical charts + process list | No treemap view |
| zenith | Rust | Charts with GPU support | No treemap view |
| ytop | Rust | Minimal charts | Unmaintained, no treemap |
| gtop | JS | Dashboard-style | No treemap, higher overhead |
| bpytop/btop++ | Python/C++ | Beautiful dashboard | No treemap view |
| KSysGuard | C++ (GUI) | Has treemap view | Desktop GUI, not terminal |

**treetop's unique value**: the only terminal-based system monitor with a treemap visualization. This fills a genuine gap — spatial visualization of resource consumption is objectively more informative than sorted lists for understanding relative magnitude at a glance.
