# Contributing

## Setup

1. Install stable Rust toolchain.
2. Clone repository.
3. Run:

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Pull Requests

- Keep changes focused and small.
- Add/update tests for behavior changes.
- Keep CI green on Linux/macOS/Windows.

## Commit and release notes

- Use clear commit titles.
- Mention user-visible changes in PR description.
