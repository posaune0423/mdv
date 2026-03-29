# Current Task

- [x] Inspect the current interactive rendering path and identify the latest regressions from user feedback
- [x] Add or update tests that prove first-frame rendering does not eagerly execute off-screen Mermaid work
- [x] Stop re-sending visible graphics on every redraw and cache Kitty image transfers/placements
- [x] Defer Mermaid rasterization until needed so startup is fast again
- [x] Re-run fmt/test/clippy and TTY smoke checks, then update this review

## Review

- Latest correction: the terminal-native rewrite improved fidelity, but the user still sees sluggish scrolling and a slower startup than before.
- Primary suspected root causes:
- `render_document()` eagerly renders Mermaid PNGs for the whole document before the first frame.
- `draw()` re-encodes and re-transmits visible PNG payloads on each redraw instead of transmitting once and placing cheaply.
- Target outcome for this pass:
- first frame appears without waiting on off-screen Mermaid diagrams
- scrolling while a graphic is visible does not resend the full image payload every frame
- Keep the terminal-native text path, but make the graphics path visible-first and cache-aware.
- Implemented changes:
- interactive Mermaid blocks now stay deferred in `render_document()` and are rendered only when they become visible during idle time
- resolved Mermaid graphics now resize their reserved blank region to the actual aspect ratio, preventing stretched diagrams
- Kitty graphics transfers are split into transmit-once plus place-per-draw, so scroll no longer base64-encodes and re-sends the same PNG payload every frame
- the rich fixture now references a normal `png` asset instead of `ppm`
- Verification:
- `cargo fmt --all` passed
- `cargo test --workspace --all-targets --all-features` passed
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed
- TTY smoke check: `cargo run --quiet -- examples/rich_markdown.md` started and exited immediately on `q`
