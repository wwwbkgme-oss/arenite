# Contributing to Arenite Engine

## Build

```bash
# Linux deps (Ubuntu/Debian)
sudo apt-get install -y \
  build-essential libvulkan-dev libegl1-mesa-dev \
  libx11-dev libxcb-xfixes0-dev libxkbcommon-dev \
  libwayland-dev pkg-config

# macOS — Xcode CLI tools + Rust
xcode-select --install

cargo build --workspace          # debug
cargo build --workspace --release
cargo run -p arenite-game        # run the client
cargo run -p arenite-server      # run the server
```

## Tests

```bash
cargo test --workspace           # all unit + integration tests
cargo test -p arenite-sim        # sim tests only
```

## Code style

- `cargo fmt --all` before every commit
- `cargo clippy --workspace -- -D warnings` must pass
- No `unwrap()` in library code — propagate with `?` or return a safe default
- Use `SAFETY:` comments for every `unsafe` block
- Prefer descriptive variable names; avoid single-letter names except in
  tight math loops where the convention is universal

## PR checklist

- [ ] `cargo check --workspace` → 0 errors
- [ ] `cargo clippy --workspace -- -D warnings` → 0 errors
- [ ] `cargo test --workspace` → all tests pass
- [ ] No new `unwrap()` in library crates
- [ ] CHANGELOG.md `[Unreleased]` section updated
- [ ] Commit message format: `type(scope): summary`
  (`feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`)

## Architecture

See `docs/architecture.md` (T-044) for the full crate dependency graph.

Key design rules:
- `arenite-core` has **no game deps** — only std + glam + serde
- `arenite-sim` has no rendering deps — pure logic
- `arenite-render` depends on `arenite-sim` but not on `arenite-world`
- Data flows: worldgen → sim → render; physics → sim → render

## License

Apache-2.0 — see [LICENSE](LICENSE).
