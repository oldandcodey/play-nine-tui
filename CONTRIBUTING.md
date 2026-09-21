# Contributing

Thanks for taking an interest in **play-nine-tui**.

## Build

```bash
git clone https://github.com/bpelleti/play-nine-tui.git
cd play-nine-tui
cargo build --release
./play
# or: cargo run
```

Requires Rust edition 2021 (MSRV 1.85). A color-capable terminal (`TERM=xterm-256color`) is recommended.

## Test

```bash
cargo test
```

Unit tests cover scoring, leaders, sayings/scenes, and JSON persistence. The `tests/json_shape.rs` integration test checks the on-disk history shape.

## Coding notes

- Keep behavior SSH-safe: fixed-width ASCII / common symbols; avoid wide emoji that break column alignment.
- UI lives in `src/ui.rs`; game logic in `src/app.rs`; JSON I/O in `src/persist.rs`.
- Do not commit `data/*.json` (runtime history). Keep `data/.gitkeep`.
- Prefer small, focused PRs. Match existing style (rustfmt defaults are fine).

## License

By contributing you agree your changes are licensed under the MIT License (see `LICENSE`).
