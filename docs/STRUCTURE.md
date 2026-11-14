# STRUCTURE: `mdv`

| Item | Value |
| --- | --- |
| Version | 1.0 |
| Status | Active |
| Product | `mdv` |
| Document Basis | Repository audit on 2026-03-29 |

## 1. Root Layout

The main top-level entries are:

```text
.
├── .agents/
├── .cargo/
├── .github/
├── bin/
├── docs/
├── examples/
├── scripts/
├── src/
├── tests/
├── Cargo.toml
├── Makefile
├── README.md
└── rust-toolchain.toml
```

## 2. Directory Responsibilities

| Path | Role |
| --- | --- |
| `docs/` | Product, technical, and structural documentation, plus initial design notes |
| `.agents/memory/` | Agent task memory and lessons |
| `.github/workflows/` | CI workflow |
| `bin/` | Git-tracked `mdv` binary used for installation and updates |
| `scripts/` | Distribution install script |
| `examples/` | Rich Markdown fixtures and sample images |
| `src/` | Main implementation |
| `tests/` | Unit / integration / e2e tests |

## 3. `src/` Layout

```text
src/
├── app/
├── cli/
├── core/
├── io/
│   └── webkit_snapshot/
├── render/
│   ├── assets/
│   └── github_html/
├── support/
├── ui/
│   └── terminal/
├── lib.rs
└── main.rs
```

### 3.1 Entry Points

| File | Role |
| --- | --- |
| `src/main.rs` | Process entry point; only parses the CLI and calls `app::run()` |
| `src/lib.rs` | Public module definitions and minimal smoke tests |

### 3.2 `src/cli/`

| File | Role |
| --- | --- |
| `src/cli/mod.rs` | `parse()` and startup message helpers |
| `src/cli/args.rs` | `MdvArgs`, `Theme`, and CLI schema |

### 3.3 `src/app/`

| File | Role |
| --- | --- |
| `src/app/mod.rs` | Startup orchestration, TTY detection, and interactive / headless branching |

`app` is effectively the composition root, bringing together `cli`, `core`, `io`, `render`, and `ui`.

### 3.4 `src/core/`

| File | Role |
| --- | --- |
| `src/core/config.rs` | `AppConfig`, `MermaidMode` |
| `src/core/document.rs` | Intermediate representations such as `Document`, `BlockKind`, and inline styles |
| `src/core/layout.rs` | Viewport and visible-range calculations |
| `src/core/theme.rs` | TUI color tokens |
| `src/core/diff.rs` | Block diffs between `Document` instances |

This layer holds the foundational types for product logic and is used by both renderers and the UI.

### 3.5 `src/render/`

| File | Role |
| --- | --- |
| `src/render/markdown.rs` | Normalize the `comrak` AST into `Document` |
| `src/render/markdown_pipeline.rs` | Shared GFM option definitions |
| `src/render/text.rs` | Headless renderer and structured text layout |
| `src/render/svg.rs` | SVG rendering helpers |
| `src/render/mod.rs` | Public render module interface |

#### `src/render/github_html/`

| File | Role |
| --- | --- |
| `src/render/github_html/mod.rs` | Entry point for GitHub HTML generation |
| `src/render/github_html/styles.rs` | Embed GitHub CSS and font-face declarations |
| `src/render/github_html/mermaid.rs` | Replace Mermaid code blocks before rendering |
| `src/render/github_html/postprocess.rs` | Format code blocks, inject alert icons, and retint tokens |
| `src/render/github_html/tests.rs` | Focused tests for the render pipeline |

#### `src/render/assets/`

- GitHub light / dark CSS
- Embedded fonts

### 3.6 `src/io/`

| File | Role |
| --- | --- |
| `src/io/fs.rs` | Read files and retrieve modification times |
| `src/io/browser.rs` | Launch the default browser |
| `src/io/image_decoder.rs` | Load local images, convert to PNG, and retrieve dimensions |
| `src/io/kitty_graphics.rs` | Kitty graphics escape sequence encoder |
| `src/io/mermaid_cli.rs` | Run the Mermaid CLI and cache results |
| `src/io/mod.rs` | Public IO module interface |

#### `src/io/webkit_snapshot/`

| File | Role |
| --- | --- |
| `src/io/snapshot.rs` | Select the HTML raster backend for each OS |
| `src/io/chromium_snapshot/mod.rs` | CDP rendering, readiness checks, and PNG capture with Edge/Chrome on Windows |
| `src/io/webkit_snapshot/mod.rs` | Public API for HTML snapshots |
| `src/io/webkit_snapshot/diagnostics.rs` | Snapshot report types |
| `src/io/webkit_snapshot/paths.rs` | Calculate the read-access root and temporary workspace |
| `src/io/webkit_snapshot/snapshot.swift` | WebKit snapshot helper compiled at build time |
| `src/io/webkit_snapshot/worker.rs` | Manage the private Unix socket and shared helper startup, reconnection, and deadlines |
| `src/io/webkit_snapshot/tests.rs` | Tests for paths and diagnostics |

### 3.7 `src/ui/`

| File | Role |
| --- | --- |
| `src/ui/mod.rs` | Public UI module interface |
| `src/ui/page_graphics.rs` | Slice full-page PNGs into viewports |

#### `src/ui/terminal/`

| File | Role |
| --- | --- |
| `src/ui/terminal/mod.rs` | `TerminalViewer`, event loop, drawing, reload, and status line |
| `src/ui/terminal/layout.rs` | Article width, cell metrics, and graphic page construction |
| `src/ui/terminal/graphics.rs` | Graphics placement and visible-command collection |
| `src/ui/terminal/graphics/resident.rs` | Keep up to two images resident, prefetch, and update source crops |
| `src/ui/terminal/scroll.rs` | Logical scroll position and time-based interpolation of the visual position |
| `src/ui/terminal/highlight.rs` | Terminal representation of code highlighting |
| `src/ui/terminal/tests.rs` | Tests for the viewer and placement |

### 3.8 `src/support/`

| File | Role |
| --- | --- |
| `src/support/tracing.rs` | Tracing initialization |

## 4. Separation of Responsibilities Along the Execution Flow

### 4.1 Startup

1. `src/main.rs`
2. `src/cli/args.rs`
3. `src/app/mod.rs`

### 4.2 Parse and Normalize

1. `src/io/fs.rs`
2. `src/render/markdown.rs`
3. `src/core/document.rs`

### 4.3 Interactive Graphic Rendering

1. `src/render/github_html/mod.rs`
2. `src/io/webkit_snapshot/mod.rs`
3. `src/ui/page_graphics.rs`
4. `src/ui/terminal/mod.rs`
5. `src/io/kitty_graphics.rs`

### 4.4 Headless Rendering

1. `src/render/text.rs`
2. `src/io/image_decoder.rs`
3. `src/io/mermaid_cli.rs`

## 5. `tests/` Layout

```text
tests/
├── e2e.rs
├── integration.rs
├── unit.rs
├── e2e/
├── integration/
└── unit/
    └── render_pipeline/
```

### 5.1 `tests/unit/`

Responsibilities:

- Parser normalization
- Layout calculations
- Image decoder
- Mermaid CLI adapter
- SVG / terminal rendering
- GitHub HTML / WebKit snapshot diagnostics

### 5.2 `tests/integration/`

Responsibilities:

- Observable CLI contract
- Help output
- Headless render feature coverage

### 5.3 `tests/e2e/`

Responsibilities:

- End-to-end execution with richer fixtures
- Execution scenarios involving local images and a fake Mermaid renderer

## 6. Supporting Files

| File | Role |
| --- | --- |
| `README.md` | Introduction and usage instructions for end users |
| `Cargo.toml` | Crate metadata, dependencies, and lint policy |
| `build.rs` | Precompile the macOS Swift helper and specify the target architecture |
| `Cargo.lock` | Dependency lockfile |
| `rust-toolchain.toml` | Pin the Rust version |
| `.cargo/config.toml` | Cargo aliases |
| `Makefile` | Build / test / lint shortcuts and `build-tracked-bin` for reproducing CI packaging |
| `bin/mdv` | Tracked binary used by the install script and `mdv update` |
| `scripts/install.sh` | Download and install `bin/mdv` from GitHub `main` |
| `.github/workflows/ci.yml` | CI for fmt / clippy / test; also generates and commits `bin/mdv` after pushes to `main` |

## 7. Notable Design Choices

A key aspect of this repository is that `render` and `io` are central to the product's value rather than merely supporting layers.

- `core` provides shared models
- `render` handles Markdown and GitHub-like fidelity
- `io` connects to external systems: the browser, Mermaid, WebKit, and the Kitty protocol
- `ui` provides the interactive terminal experience
- `app` assembles these components

The structure emphasizes the rendering pipeline of a viewer product over strict clean architecture.

## 8. Areas Suited to Further Structural Work

- Separate the link UX that depends on `DocumentMeta.links.first()` into stateful navigation
- Extract the interactive fallback as a strategy within `ui::terminal`
- Abstract platform-specific snapshot backends so they can extend beyond `io::webkit_snapshot`
