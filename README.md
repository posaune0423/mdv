<div align="center">

# mdv

**High-fidelity Markdown viewer for [Ghostty](https://ghostty.org/) and [Kitty](https://sw.kovidgoyal.net/kitty/).**

[![CI](https://github.com/posaune0423/mdv/actions/workflows/ci.yml/badge.svg)](https://github.com/posaune0423/mdv/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
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
- **Rust toolchain**: **1.92+** (matches CI; edition 2024) — only if you build from source (see Homebrew / `cargo install` below).

## Installation

Prebuilt binaries are attached to [GitHub Releases](https://github.com/posaune0423/mdv/releases) (Linux x86_64 / Linux arm64 / macOS Intel / macOS Apple Silicon). Each archive contains a single `mdv` executable at the top level.

### curl (recommended for end users)

Install into `$HOME/.local/bin` (override with `MDV_INSTALL_DIR`):

```bash
curl --proto '=https' --tlsv1.2 -LsSf \
  https://raw.githubusercontent.com/posaune0423/mdv/main/scripts/install.sh | sh
```

Pin to a release tag instead of `main` when you want a fixed installer revision:

```text
https://raw.githubusercontent.com/posaune0423/mdv/v0.1.0/scripts/install.sh
```

Verify checksums when you need supply-chain guarantees: every release publishes `SHA256SUMS` next to the archives.

### Homebrew (third-party tap)

If this repository contains `Formula/mdv.rb`, you can install from a tap (not from homebrew-core):

```bash
brew tap posaune0423/mdv https://github.com/posaune0423/mdv
brew install mdv
```

### mise

Install straight from GitHub Releases using the built-in `github` backend (zero extra plugins):

```bash
mise use -g github:posaune0423/mdv
```

To type `mise install mdv` / `mise use -g mdv@latest`, register a short name once in `~/.config/mise/config.toml`:

```toml
[tool_alias]
mdv = "github:posaune0423/mdv"
```

Upstream registry support (so `mise install mdv` works without `tool_alias`) is tracked in the [mise](https://github.com/jdx/mise) / [registry](https://github.com/jdx/mise/blob/main/registry.toml) project—contributions welcome.

### Build from source (Cargo)

```bash
cargo install --path . --locked --force
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

## Changelog

Release notes and version history live in [CHANGELOG.md](./CHANGELOG.md).

## Contributing

Issues and pull requests are welcome. Please run `make ci` before opening a PR so formatting, Clippy, and tests match what GitHub Actions enforces.

### Changelog & releases (Changesets)

User-visible changes should include a **changeset** so the [Changesets](https://github.com/changesets/changesets) automation can version `package.json`, `Cargo.toml`, and [CHANGELOG.md](./CHANGELOG.md) together.

1. Install the **[Changeset bot](https://github.com/apps/changeset-bot)** on this GitHub org/repo (same idea as [this Zenn walkthrough](https://zenn.dev/hayato94087/articles/a0598a8816f25c#4.changeset-bot%E3%82%92%E8%A8%AD%E5%AE%9A)).
2. In **Settings → Actions → General → Workflow permissions**, enable **Allow GitHub Actions to create and approve pull requests** so the [Changesets workflow](.github/workflows/changeset-release.yml) can open the “Version Packages” PR.
3. Locally: `npm ci` then `npx changeset` (or use the bot on a PR).

After merges to `main`, the bot opens/updates a release PR; merging it runs `changeset publish`, creates the `mdv@x.y.z` tag, and the [Release](.github/workflows/release.yml) workflow uploads binaries. GitHub Releases themselves are created by that Rust workflow only (`createGithubReleases: false` on the Changesets action avoids duplicate releases).

### Contributors

Everyone who lands a change shows up automatically on the [GitHub contributors graph](https://github.com/posaune0423/mdv/graphs/contributors). Thank you for helping improve mdv.

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

| Command | Purpose |
|---------|---------|
| `make fmt` / `make fmt-check` | `rustfmt` |
| `make lint` | `clippy` with warnings denied |
| `make test` | All tests |
| `make test-unit` / `test-integration` / `test-e2e` | Split suites |

## License

mdv is licensed under the **MIT License**. See [LICENSE](./LICENSE) for the full text.

```text
MIT License

Copyright (c) 2026 mdv contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

<div align="center">

<br/>

<sub>MIT © <a href="https://github.com/posaune0423/mdv/graphs/contributors">mdv contributors</a></sub>

</div>
