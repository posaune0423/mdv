# TECH: `mdv`

| Item | Value |
| --- | --- |
| Version | 1.0 |
| Status | Active |
| Product | `mdv` |
| Language | Rust 2024 |
| Toolchain | Rust `1.92.0` |
| Document Basis | Repository audit on 2026-03-29 |

## 1. Technical Overview

`mdv` is a single-binary CLI written in Rust. It first normalizes Markdown into a `Document`, then branches into two output paths.

- Interactive mode: snapshot a page styled like GitHub HTML to PNG and display it in the terminal using the Kitty graphics protocol
- Non-interactive mode: generate plain-text rendering from `Document` and write it to stdout

The entry point is simple: `src/main.rs` only calls `mdv::cli::parse()` and `mdv::app::run()`.

## 2. Main Dependencies

| Crate | Purpose |
| --- | --- |
| `clap` | CLI argument parsing |
| `anyhow` / `thiserror` | Add error context |
| `comrak` | GFM-based Markdown parsing and HTML output |
| `crossterm` | TTY control, keyboard input, and alternate screen |
| `image` | Image decoding, PNG conversion, and dimension retrieval |
| `resvg` | SVG rasterization |
| `syntect` | Code block syntax highlighting |
| `serde` / `serde_json` | JSON processing for snapshot diagnostics |
| `tracing` / `tracing-subscriber` | Tracing for startup, rendering, and external commands |

## 3. Runtime Modes

### 3.1 Headless Mode

Condition:

- Either `stdout` or `stdin` is not a TTY

Behavior:

- Use `render::text::render_plain_text()`
- Do not use the alternate screen
- Return metadata-based degraded representations for images / Mermaid

### 3.2 Interactive Mode

Conditions:

- Both `stdout` and `stdin` are TTYs
- The terminal is identified as Ghostty, Kitty, or WezTerm, or is a Unix terminal that answers the Kitty graphics support query

Behavior:

- Create a viewer with `TerminalViewer::try_new()`
- Enter raw mode and the alternate screen
- Draw the graphic page and handle scrolling, reloading, and opening links in the keyboard input loop

Constraints:

- The current implementation requires graphic page generation and has no runtime fallback on failure
- `io::snapshot` selects WebKit/TIFF on macOS and Chromium/PNG on Windows. Windows image display uses WezTerm

## 4. Data Flow

### 4.1 Parse Pipeline

1. `app::run()` reads the file contents
2. `render::markdown::parse_document()` traverses the `comrak` AST
3. The AST is normalized into `core::document::Document` and its `BlockKind` values
4. `DocumentMeta` stores the title, links, and source length

`Document` is the application's shared intermediate representation and the starting point for both headless and interactive modes.

### 4.2 Interactive Render Pipeline

1. `render::github_html::build_github_html()` generates GitHub-like HTML
2. `replace_mermaid_code_blocks()` replaces Mermaid code fences with SVG or fallback HTML before rendering
3. The WebKit worker converts HTML into TIFF and a diagnostics report
4. `ui::page_graphics::build_graphic_page()` produces RGBA data and the display size
5. `ui::terminal::graphics::resident` retains up to two contiguous images covering the visible region and updates only the source crop during normal frames
6. `ui::terminal` batches input on an approximately 60 Hz schedule and updates the screen using brief pixel interpolation and synchronized output. The next image is encoded as a lossless PNG in the background, and all chunks are transmitted consecutively

### 4.3 Headless Render Pipeline

1. `render_plain_text()` traverses `Document` block by block
2. Blocks are formatted as plain text
3. Images / Mermaid / footnotes / tables are converted into readable degraded representations
4. Output is written to stdout as UTF-8

## 5. Module Responsibilities

| Module | Responsibility |
| --- | --- |
| `cli` | `MdvArgs` definitions, theme enum, and argument parsing |
| `app` | Startup orchestration, TTY detection, and render mode selection |
| `core` | `Document`, `BlockKind`, theme tokens, layout helpers, and diffs |
| `render::markdown` | GFM parsing and `Document` normalization |
| `render::text` | Headless display and structured text layout |
| `render::github_html` | HTML generation using GitHub CSS and Mermaid embedding |
| `render::svg` | SVG rendering helpers and related tests |
| `io::fs` | Read documents and retrieve modification times |
| `io::image_decoder` | Resolve local images and convert them to PNG |
| `io::mermaid_cli` | Invoke `mmdc` / `npx @mermaid-js/mermaid-cli` and cache results |
| `io::webkit_snapshot` | HTML snapshots using a Swift helper |
| `io::browser` | Launch the default browser |
| `io::kitty_graphics` | Generate Kitty graphics escape sequences |
| `ui::terminal` | Event loop, status line, and graphics placement |
| `ui::page_graphics` | Crop viewports from full-page PNGs |

## 6. External Dependencies and System Requirements

### 6.1 Mermaid

`MermaidCliRenderer::from_env()` searches for Mermaid in this order:

1. `MDV_MERMAID_CMD`
2. `mmdc`
3. `npx -y @mermaid-js/mermaid-cli`

Rendering results are stored in the cache directory.

### 6.2 WebKit Snapshot

HTML snapshots use `WKWebView` through a Swift helper. `build.rs` compiles `snapshot.swift` for macOS and embeds the executable in the Rust binary. `worker.rs` manages a private Unix socket for each user and helper version, reusing the WebKit process and view. The worker exits 120 seconds after its last request. Documents and assets are reread on every request, preserving per-document read-access roots, image/font readiness checks, and diagnostics. The rendering deadline is 25 seconds; after a failure, the view is discarded so the next document can be rendered.

Internal transfers for interactive display use uncompressed TIFF, which Rust converts to RGBA once and retains. Only a lossless PNG of the visible viewport is sent to the terminal. Avoiding full-page PNG compression/decompression and full-page decoding on every scroll reduces startup and scrolling costs without changing resolution, color, transparency, or crop geometry. The existing `render_html_to_png` API returns PNG and diagnostics.

Outputs:

- `snapshot.png` (`render_html_to_png`) or `snapshot.tiff` (interactive display)
- `snapshot-report.json`

Diagnostics include:

- image readiness
- Mermaid metrics
- typography metrics
- asset failure diagnostics

### 6.3 Browser Open

`io::browser` selects the default-browser launch command for the OS. It is used only when `o` is pressed in interactive mode.

## 7. Themes and Appearance

There are only two themes: `Theme::Light` and `Theme::Dark`.

The interactive path combines:

- GitHub light / dark CSS assets
- Embedded `Mona Sans VF` and `Monaspace Neon Var`
- A `syntect` syntax highlighting theme

The headless path has `ThemeTokens`, but its final output is primarily plain text, so theme differences are less pronounced than in interactive mode.

## 8. Build, Verification, and Distribution

### 8.1 Local Development

- `cargo build --release`
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features`

Equivalent entry points are also defined in `Makefile`.

### 8.2 CI

GitHub Actions `ci.yml` runs:

- format check
- clippy
- test

The Rust toolchain is pinned to `1.92.0`.

### 8.3 Distribution

There is currently no release automation. The distribution contract consists of two paths:

- `scripts/install.sh` downloads the tracked `bin/mdv` from GitHub `main` and places it in the installation directory
- `mdv update` compares the tracked binary for the current OS and replaces it when its contents differ and GitHub `main` has a newer version. Equal versions are not updated, so publishing a new binary requires incrementing the version in `Cargo.toml`.

The repository's `bin/mdv` is the artifact used for installation and updates. After a push to `main`, the macOS job in `ci.yml` generates it automatically and commits it only when necessary. Locally, `make build-tracked-bin` reproduces the same packaging path, but CI, rather than manual work, owns updates to the tracked binary.

## 9. Test Strategy

Tests are divided into three layers:

- `tests/unit`: unit checks for the parser, renderer, snapshots, terminal helpers, Mermaid adapter, and related components
- `tests/integration`: checks for CLI and output contracts
- `tests/e2e`: checks for execution paths involving rich fixtures and asset resolution

Particularly important areas are:

- headless rendering contract
- GitHub HTML typography fidelity
- local image rendering
- Mermaid diagnostics
- terminal scroll / graphics placement

## 10. Known Technical Constraints

1. Windows image display requires a supported terminal (WezTerm) and an installed Edge/Chrome browser
2. Native compositor rendering and input feel require separate verification for each terminal and OS
3. Opening links always uses the first link; there is no link focus state
4. Terminal detection accepts Ghostty / Kitty / WezTerm and Unix terminals that answer the Kitty graphics support query
5. The README's platform statement differs from the current implementation's runtime contract

## 11. Potential Improvements

- Provide a snapshot backend that works on Linux or a structured TUI fallback
- Use `Document`'s `revision` and `SourceSpan` for reload diffs
- Restore the interactive path's fallback to `render::text::render_document()`
- Extend `DocumentMeta` and UI state when adding multi-link navigation or a search UI

## Windows backend

`io::snapshot::render_html_to_raster` selects the existing WebKit backend or `io::chromium_snapshot` according to the OS. Windows uses an installed Edge/Chrome browser with a dedicated profile and sandboxing enabled, capturing lossless PNGs with a fixed CSS viewport and deviceScaleFactor=2. It neither connects to the normal browser profile nor caches completed images.

On Windows, the visible region is sent to WezTerm as a PNG over OSC1337, preserving fractional-position scroll interpolation. This does not depend on ConPTY passing Kitty image commands through. Windows Terminal is not supported by this image transport. The resident-image approach used by Ghostty/Kitty on macOS remains unchanged.

Windows updates use `bin/mdv.exe` and an API capable of replacing a running executable. Unsupported OS/architecture combinations never download a macOS binary.
