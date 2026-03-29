# Lessons

- Do not call an implementation a rich markdown viewer until it has been validated in a real TTY and is visibly distinct from raw markdown output.
- Do not use a repaint-on-every-poll loop for an interactive graphics viewer; redraw must be driven by state changes and placement diffs or the result will flicker and hide input bugs.
- Typography and code rendering are part of fidelity; if the viewer still looks small or flat next to GitHub, revisit font scale and syntax highlighting before calling the design pass complete.
- First-frame latency is user-visible product quality; avoid per-frame expensive initialization such as loading fonts or syntax resources inside the hot render path.
- When scroll input can queue faster than rendering, drain and coalesce pending terminal events before issuing another heavy redraw; otherwise quit and navigation keys will appear frozen behind redraw backlog.
- A fast quit-path is not enough to validate scrolling; always verify that a rich fixture can reach its bottom section in a real TTY and that major blocks are visually transformed from raw markdown markers.
