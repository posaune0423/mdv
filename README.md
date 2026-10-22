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

The [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) is also implemented by terminals including Ghostty, Konsole, st (with a patch), Warp, wayst, WezTerm, iTerm2, and xterm.js. `mdv` enables interactive mode in Ghostty, Kitty, and WezTerm. On Windows, use WezTerm: its lossless inline-image transport works through the Windows console, while Windows Terminal does not currently support this viewer’s image path.

## Features

|                              |                                                                                                |
| ---------------------------- | ---------------------------------------------------------------------------------------------- |
| **GitHub CSS rendering**     | GitHub styles rendered by WebKit on macOS or installed Edge/Chrome on Windows |
| **Inline images**            | PNG, JPEG, GIF, WebP rendered directly in the terminal                                         |
| **Mermaid diagrams**         | Flowcharts, sequence diagrams, and more — rendered inline                                      |
| **Syntax highlighting**      | 100+ languages via [syntect](https://github.com/trishume/syntect)                              |
| **System/light/dark themes** | `system` by default, with explicit `light` / `dark` overrides                                  |
| **Watch mode**               | `--watch` auto-reloads on file change                                                          |
| **Quick update**             | `mdv update` replaces the current executable when `main`'s CI-generated `bin/mdv` changes      |
| **Pipe-friendly**            | Plain-text fallback when stdout is not a TTY — works in CI and scripts                         |
| **Vim navigation**           | `j`/`k`/`g`/`G`/PageUp/PageDown                                                                |
| **Smooth scrolling**        | Mouse wheel, trackpad, and keys with smooth motion and responsive input                        |

## Quick start

**Install** (into `$HOME/.local/bin` by default):

```bash
curl -fsSL https://raw.githubusercontent.com/posaune0423/mdv/main/scripts/install.sh | sh
```

This installs the CI-generated macOS Apple Silicon [`bin/mdv`](bin/mdv) artifact from `main`.

On **Windows x64**, install [WezTerm](https://wezterm.org/install/windows.html) and Microsoft Edge (or Chrome), then run in PowerShell:

```powershell
irm https://raw.githubusercontent.com/posaune0423/mdv/main/scripts/install.ps1 | iex
```

The Windows installer downloads [`bin/mdv.exe`](bin/mdv.exe) to `$env:LOCALAPPDATA\mdv\bin`, adds that directory to your user PATH, and requires no administrator access. Open a new WezTerm window and run `mdv README.md`. Set `MDV_INSTALL_DIR` to choose another directory. `mdv update` selects the correct binary for macOS Apple Silicon or Windows x64.

Edge/Chrome must be installed; no browser is downloaded by `mdv`. To choose an executable explicitly:

```powershell
$env:MDV_BROWSER = 'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'
mdv README.md
```

Windows Terminal and the legacy console can still use the plain-text pipeline (`Get-Content README.md | mdv -`); image viewing requires WezTerm. Windows ARM64 native binaries are not distributed.

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
| Mouse wheel / trackpad | Smooth scroll      |
| `r`                   | Reload file          |
| `o`                   | Open link in browser |
| `q`                   | Quit                 |

## Requirements

- **Terminal**: macOS: [Ghostty](https://ghostty.org/), [Kitty](https://sw.kovidgoyal.net/kitty/), or [WezTerm](https://wezterm.org/); Windows: WezTerm
- **Protocol ecosystem**: The [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) is also implemented by Ghostty, Konsole, st (with a patch), Warp, wayst, WezTerm, iTerm2, and xterm.js
- **Rich rendering**: macOS uses its system WebKit; Windows x64 uses installed Microsoft Edge or Chrome. Both produce lossless page images. Linux remains a plain-text/headless target.
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
| Graphic / snapshot issues          | macOS uses WebKit; Windows requires installed Edge/Chrome and WezTerm. See [docs/TECH.md](docs/TECH.md).                                           |

## Documentation

- [llm.txt](llm.txt) — Agent-oriented install & CLI summary
- [GFM features & authoring](docs/MARKDOWN.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Development & contributing](docs/DEVELOPMENT.md)

## Contributing

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). Run `make ci` before sending a PR.

## License

Released under the [MIT License](LICENSE).
