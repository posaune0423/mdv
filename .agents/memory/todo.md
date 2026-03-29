# Current Task

- [x] Reproduce the remaining fidelity regressions in the interactive renderer using the rich fixture
- [x] Add or update tests for terminal-native block rendering, scroll reachability, and fixture coverage
- [x] Replace the full-text viewport PNG path with terminal-native text rendering and align graphics placements
- [x] Improve block formatting for code, callouts, tables, lists, checkboxes, links, images, and Mermaid fallbacks
- [x] Re-run fmt/test/clippy and TTY smoke checks, then update this review

## Review

- Root cause: the full-text viewport PNG approach made scrolling expensive and also forced many blocks to degrade into raw-ish text overlays.
- Architecture change: interactive rendering now draws text natively in the terminal and reserves Kitty graphics placements for image and Mermaid blocks only.
- Fidelity improvements:
- Lists and checkboxes now render as bullets plus `☑` / `☐`.
- Links no longer show raw `<url>` destinations in the interactive paragraph flow.
- Tables render as box tables instead of raw pipe rows.
- Code fences render as boxed blocks with syntax-highlighted terminal output.
- Callouts render with a left accent bar and block styling instead of plain markdown markers.
- Graphics placement is horizontally aligned to the article column, fixing the left-shifted image issue.
- Mermaid now falls back to `npx @mermaid-js/mermaid-cli` when `mmdc` is absent, and child renderer output is silenced.
- Reachability verification: in a real TTY, `G` on `examples/rich_markdown.md` reached the footnote section at the bottom, confirming lower content is scrollable.
- `cargo fmt --all` passed.
- `cargo test --workspace --all-targets --all-features` passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- TTY smoke check: `cargo run --quiet -- examples/rich_markdown.md` rendered the styled first frame without Mermaid spinner leakage.
- TTY bottom check: `cargo run --quiet -- examples/rich_markdown.md`, then `G`, showed the footnotes at the bottom.
