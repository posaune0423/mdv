<div align="center">

# mdv

**High-fidelity Markdown viewer for [Ghostty](https://ghostty.org/) and [Kitty](https://sw.kovidgoyal.net/kitty/).**

[![CI](https://github.com/posaune0423/mdv/actions/workflows/ci.yml/badge.svg)](https://github.com/posaune0423/mdv/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org/)
[![Edition](https://img.shields.io/badge/edition-2024-purple.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![unsafe: forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://doc.rust-lang.org/nomicon/safe-unsafe-meaning.html)

Render GitHub Flavored Markdown in the terminal with rich typography, images, SVG, diagrams, and syntax-highlighted code blocks.

<br/>

</div>

---

## Features

| Area | Details |
|------|---------|
| **Markdown** | GFM via [Comrak](https://github.com/kivikakk/comrak)—headings, lists, tables, task lists, and more |
| **Code** | Syntax highlighting powered by [Syntect](https://github.com/trishume/syntect) |
| **Assets** | PNG, JPEG, GIF, WebP, and SVG rendering where the terminal supports it |
| **Diagrams** | [Mermaid](https://mermaid.js.org/) diagrams (optional `--no-mermaid` to disable) |
| **Themes** | `light` / `dark` (`--theme`) |
| **Workflow** | `--watch` reloads when the file changes on disk |
| **CI / Pipes** | Non-interactive use prints a plain-text rendering to stdout |

## Requirements

- **Interactive TUI**: Ghostty or Kitty (`TERM_PROGRAM` / terminal detection). Other terminals get a clear error in interactive mode.
- **Rust toolchain**: **1.92+** (matches CI; edition 2024).

## Installation

```bash
cargo install --path . --force
# or
make install-local
```

Binary name: `mdv`.

## Usage

```bash
# Open a file (default theme: light)
mdv README.md

# Dark theme + live reload
mdv --theme dark --watch ./docs/guide.md

# Disable Mermaid (placeholders only)
mdv --no-mermaid notes.md
```

**Pipelines** — when stdout is not a TTY (e.g. redirected or in CI), `mdv` writes a plain-text view instead of opening the interactive viewer:

```bash
mdv ./CHANGELOG.md | head -n 50
```

## Development

Repository root includes a small `./mdv` shell helper that rebuilds the debug binary when sources change, then runs `target/debug/mdv`.

```bash
chmod +x ./mdv   # once, if needed
./mdv ./some.md
```

Quality gates (same as CI):

```bash
make ci    # fmt-check, clippy -D warnings, full test suite
```

Or step by step:

| Command | Purpose |
|---------|---------|
| `make fmt` / `make fmt-check` | `rustfmt` |
| `make lint` | `clippy` with warnings denied |
| `make test` | All tests |
| `make test-unit` / `test-integration` / `test-e2e` | Split suites |

## License

<div align="center">

MIT © contributors

</div>
