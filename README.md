<div align="center">

# mdv

**Browser-quality Markdown, right in your terminal.**

<img src="docs/demo.gif" width="700" alt="mdv rendering markdown in Ghostty" />

[![CI](https://github.com/posaune0423/mdv/actions/workflows/ci.yml/badge.svg)](https://github.com/posaune0423/mdv/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)

</div>

`mdv` is a blazing-fast Markdown viewer built in Rust. With the [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/), it renders rich, beautiful Markdown natively in your terminal, including typography, images, diagrams, syntax highlighting, and GFM features like callouts.

If you live in [Ghostty](https://ghostty.org/) or [Kitty](https://sw.kovidgoyal.net/kitty/), you no longer need to leave the terminal to read a README.

The [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) is also implemented by terminals including Ghostty, Konsole, st (with a patch), Warp, wayst, WezTerm, iTerm2, and xterm.js. `mdv` currently enables interactive mode in Ghostty and Kitty.

## Features

|                              |                                                                                                |
| ---------------------------- | ---------------------------------------------------------------------------------------------- |
| **GitHub CSS rendering**     | WebKit snapshots with GitHub's actual stylesheets — what you see on github.com is what you get |
| **Inline images**            | PNG, JPEG, GIF, WebP rendered directly in the terminal                                         |
| **Mermaid diagrams**         | Flowcharts, sequence diagrams, and more — rendered inline                                      |
| **Syntax highlighting**      | 100+ languages via [syntect](https://github.com/trishume/syntect)                              |
| **System/light/dark themes** | `system` by default, with explicit `light` / `dark` overrides                                  |
| **Watch mode**               | `--watch` auto-reloads on file change                                                          |
| **Quick update**             | `mdv update` replaces the current executable when `main`'s CI-generated `bin/mdv` changes      |
| **Pipe-friendly**            | Plain-text fallback when stdout is not a TTY — works in CI and scripts                         |
| **Vim navigation**           | `j`/`k`/`g`/`G`/PageUp/PageDown                                                                |

## Quick start

**Install** (into `$HOME/.local/bin` by default):

```bash
curl -fsSL https://raw.githubusercontent.com/posaune0423/mdv/main/scripts/install.sh | sh
```

This installs the CI-generated [`bin/mdv`](bin/mdv) artifact from `main`. If that binary does not match your host platform, build from source instead.

**Run** (in Ghostty or Kitty):

```bash
mdv README.md
```

## Usage

```bash
mdv README.md                           # system theme (default)
mdv --theme dark notes.md               # dark theme
mdv --watch docs/guide.md               # auto-reload on save
mdv --no-mermaid spec.md                # skip Mermaid rendering
mdv update                              # replace this mdv if main/bin/mdv changed
mdv --version                           # print the current version
mdv ./README.md | head -n 50            # plain-text output (pipe/CI)
```

### Keyboard shortcuts

| Key                   | Action               |
| --------------------- | -------------------- |
| `j` / `Down`          | Scroll down          |
| `k` / `Up`            | Scroll up            |
| `g`                   | Jump to top          |
| `G`                   | Jump to bottom       |
| `PageUp` / `PageDown` | Page scroll          |
| `r`                   | Reload file          |
| `o`                   | Open link in browser |
| `q`                   | Quit                 |

## Requirements

- **Terminal**: [Ghostty](https://ghostty.org/) or [Kitty](https://sw.kovidgoyal.net/kitty/) ([Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) required)
- **Protocol ecosystem**: The [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) is also implemented by Ghostty, Konsole, st (with a patch), Warp, wayst, WezTerm, iTerm2, and xterm.js
- **Rich rendering**: macOS (uses WebKit for HTML→PNG snapshots). Linux runs in headless/plain-text mode.
- **Mermaid** (optional): `mmdc` or `npx @mermaid-js/mermaid-cli`, or set `MDV_MERMAID_CMD`

## Installation

| Method             | Command / notes                                                                                                                                              |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Install script** | See [Quick start](#quick-start). Downloads the CI-generated `main` branch [`bin/mdv`](bin/mdv) and installs it into `MDV_INSTALL_DIR` or `$HOME/.local/bin`. |
| **Cargo**          | `cargo install --path . --locked --force` or `make install-local` from a clone                                                                               |

## Troubleshooting

| Symptom                            | What to try                                                                                                          |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `mdv: command not found`           | Ensure `$HOME/.local/bin` is on `PATH`, then restart the terminal.                                                   |
| `mdv update` fails                 | Confirm the current `mdv` location is writable and GitHub `main` exposes the expected [`bin/mdv`](bin/mdv) artifact. |
| Plain text instead of rich viewer  | Requires a TTY in Ghostty or Kitty. Pipes and redirects trigger headless mode by design.                             |
| Mermaid diagrams show placeholders | Install `mmdc` or use `npx @mermaid-js/mermaid-cli`. Use `--no-mermaid` to skip entirely.                            |
| Graphic / snapshot issues          | Rich rendering uses WebKit (macOS only). See [docs/TECH.md](docs/TECH.md).                                           |

## Documentation

- [llm.txt](llm.txt) — Agent-oriented install & CLI summary
- [GFM features & authoring](docs/MARKDOWN.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development & contributing](docs/DEVELOPMENT.md)

## Contributing

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). Run `make ci` before sending a PR.

## License

Released under the [MIT License](LICENSE).
