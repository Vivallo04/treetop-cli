# treetop

`treetop` is a cross-platform terminal UI process monitor that visualizes memory usage as an interactive treemap.

## Features

- Linux/macOS/Windows support
- Treemap-based process visualization
- Multiple color modes (name, memory, cpu, user, group, mono)
- Optional perf tracing instrumentation

## Install

### From source

```bash
cargo install --path .
```

### Build release binary

```bash
cargo build --release
```

## Usage

```bash
treetop
```

Common options:

```bash
# refresh every second
treetop --refresh-rate 1000

# force color support mode
treetop --color truecolor

# use categorical process-name coloring
treetop --color-mode name
```

Perf capture mode (headless):

```bash
cargo run --features perf-tracing -- --perf-capture
```

## Development

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## License

MIT. See `LICENSE`.
