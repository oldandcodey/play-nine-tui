# Play Nine TUI — rough draft scorekeeper

Create a TUI to keep score for the card game Play Nine. Track the current game and a summarized history. Players choose 9 or 18 holes, enter names and score per hole. Track holes played, scores per user, current total. Golf motif, colorized. End celebration: modest ASCII confetti + winner name (champagne/fireworks = stretch, not required for v1). Documentation + error handling. **Rust + ratatui** (preferred). Store in JSON. Runnable over Tailscale/SSH.

## Confirmed for v1
- Integer hole scores (each hole = one integer)
- Modest celebration (ASCII confetti + winner; not full fireworks polish)
- NOT a full card-play engine — scorekeeper only
- No network multiplayer
- Persistence: `data/play_nine.json` (CWD-relative; dir created at runtime)

## Deliverable layout
```
/workspace/play-nine-tui/
  Cargo.toml
  src/main.rs
  src/app.rs
  src/ui.rs
  src/persist.rs
  README.md
  SPEC.md
  data/          # gitignored JSON; .gitkeep present
```

## Acceptance
1. `cargo build --release` succeeds
2. Flow: new game → names → hole scores → totals → finish → celebrate → quit; relaunch shows history
3. README: cargo run / release binary; TERM + SSH notes; bad input / corrupt JSON handling
