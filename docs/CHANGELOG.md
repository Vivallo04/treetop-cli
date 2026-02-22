# Changelog

All notable changes to **Treetop** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-02-22

### Added

- **Sort modes** - `SortMode` enum (Memory / CPU / Name) with `s` key to cycle; default configurable via `[general] default_sort`
- **Full TOML configuration** - `~/.config/treetop/config.toml` with `[general]`, `[treemap]`, `[colors]`, and `[keybinds]` sections
- **Customizable keybinds** - 12 remappable actions via `[keybinds]` config section; arrow keys and Ctrl+C remain hardwired
- **`ResolvedKeybinds` system** - resolves string config values to `KeyCode` at startup via `parse_key()`
- **Help overlay** - `?` key opens a centered modal showing all current keybinds; press `?` or `Esc` to dismiss
- **`ProcessState` enum** - typed process states (Running, Sleeping, Stopped, Zombie, Idle, Unknown) replacing raw `String` status field
- **Platform-specific process introspection**:
  - **Linux**: cgroup names from `/proc/<pid>/cgroup`, priority from `/proc/<pid>/stat`, I/O bytes from `/proc/<pid>/io`
  - **macOS**: app bundle name via `libproc`, priority via `libc::getpriority`, I/O returns `None` gracefully
  - **Windows**: `GetPriorityClass` and `GetProcessIoCounters` via `windows-sys`
- **`BorderStyle::Thick` and `BorderStyle::None`** variants with `has_border()` helper method
- **Help overlay module** (`src/ui/help.rs`) with centered modal rendering
- **Cross-compilation CI** - `cross-check` job for `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-gnu`
- **Release targets** - 6 platform builds: x86_64-linux-gnu, x86_64-linux-musl, aarch64-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin, x86_64-windows-msvc
- **131 tests** across lib, binary, integration, snapshot, and layout test crates

### Changed

- `ProcessInfo.status` type changed from `String` to `ProcessState` enum
- `TreemapItem.id` / `TreemapRect.id` renamed to `.pid` throughout the codebase
- Selection bar now displays PID prefix: `[1234] firefox     512 MB`
- `compute_layout()` uses sort-mode-aware ordering instead of always sorting by memory
- `map_key_normal()` uses configurable `ResolvedKeybinds` comparisons instead of hardcoded match arms
- `Ctrl+C` always quits regardless of `InputMode` (hardwired safety mechanism)
- Arrow keys remain hardwired and are not configurable

### Removed

- **Nested recursive treemap** (`nested-treemap` feature flag) - implemented but removed due to poor visual quality at typical terminal sizes

## [0.1.0] - 2025-02-13

### Added

- **Squarified treemap visualization** - Bruls et al. 2000 algorithm with area conservation and containment guarantees
- **System data collection** via `sysinfo` crate - CPU, memory, swap, load average, per-process stats
- **Single-threaded tokio event loop** with `tokio::select!` multiplexing key, tick, and animate events
- **Process filtering** - `/` key enters filter mode with incremental text search on name and command
- **Process killing** - `k` for SIGTERM, `K` for SIGKILL with status feedback
- **Zoom into process children** - `Enter` to zoom, `Esc` to zoom out, breadcrumb trail
- **Six color modes** - ByName, ByMemory, ByCpu, ByUser, ByGroup, Monochrome
- **Three theme presets** - Vivid (dark), Pastel (dark), Light
- **Sparkline history** - per-process memory/CPU ring buffer with configurable length
- **Animated layout transitions** - 5-frame lerp between old/new layouts (40ms per frame, ~200ms total)
- **Count-cap grouping** - `max_visible_procs` (default 25) caps visible rectangles; remaining grouped as "Other"
- **4-pass seam-based rendering pipeline** - backgrounds, Unicode box-drawing seams, labels, selection heavy border
- **Detail panel** - `d` toggles a side panel with process info, command, and sparkline
- **Mouse selection** - click to select processes in the treemap
- **Process tree hierarchy** - parent-child relationships with subtree memory sizing
- **ANSI-256 and truecolor support** with automatic detection and `color_support` config override
- **Criterion benchmarks** for treemap layout performance
- **`insta` snapshot tests** for UI rendering
- **`proptest` property-based tests** for treemap algorithm invariants
- **Architecture boundary tests** ensuring lib crate does not import from binary crate

[0.2.0]: https://github.com/Vivallo04/treetop-cli/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Vivallo04/treetop-cli/releases/tag/v0.1.0
