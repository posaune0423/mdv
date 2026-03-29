# Current Task

- [x] Inspect the current interactive rendering path and identify the latest regressions from user feedback
- [ ] Add or update tests that prove first-frame rendering does not eagerly execute off-screen Mermaid work
- [ ] Stop re-sending visible graphics on every redraw and cache Kitty image transfers/placements
- [ ] Defer Mermaid rasterization until needed so startup is fast again
- [ ] Re-run fmt/test/clippy and TTY smoke checks, then update this review

## Review

- Latest correction: the terminal-native rewrite improved fidelity, but the user still sees sluggish scrolling and a slower startup than before.
- Primary suspected root causes:
- `render_document()` eagerly renders Mermaid PNGs for the whole document before the first frame.
- `draw()` re-encodes and re-transmits visible PNG payloads on each redraw instead of transmitting once and placing cheaply.
- Target outcome for this pass:
- first frame appears without waiting on off-screen Mermaid diagrams
- scrolling while a graphic is visible does not resend the full image payload every frame
- Keep the terminal-native text path, but make the graphics path visible-first and cache-aware.
