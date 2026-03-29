# Architecture

High-level map of how a `.md` file becomes terminal output or stdout in this repository.

## Module map

| Layer | Role (in this repo) |
|-------|---------------------|
| **`main` / `cli`** | Parse argv → `MdvArgs` |
| **`app`** | Read file, parse Markdown once, branch interactive vs headless ([`src/app/mod.rs`](../src/app/mod.rs)) |
| **`render::markdown`** | Comrak + GFM options → domain `Document` (`BlockKind` list) |
| **`render::text`** | `Document` → plain string **or** wrapped `RenderedDocument` (lines + graphic placements) |
| **`render::github_html`** | Same source → GitHub-style HTML (for snapshot / “graphic page” mode) |
| **`io`** | FS source, image decode, Mermaid CLI, WebKit HTML→PNG, Kitty graphics escape sequences |
| **`ui::terminal`** | Crossterm TTY, scroll/input loop, draw path ([`src/ui/terminal.rs`](../src/ui/terminal.rs)) |
| **`ui::page_graphics`** | Slice a full-page PNG into terminal rows for Kitty placement |

## 1. End-to-end: `.md` file → terminal or stdout

```mermaid
flowchart TB
  subgraph Entry["Entry"]
    A["mdv path/to/doc.md"] --> B["cli parse to MdvArgs"]
    B --> C["app::run"]
  end

  subgraph Load["Load and parse"]
    C --> D["FileSystemDocumentSource read"]
    D --> E["parse_document Comrak GFM"]
    E --> F["Document blocks"]
  end

  subgraph Branch["Where output goes"]
    F --> G{"stdout and stdin both TTY?"}
    G -->|no| H["render_plain_text"]
    H --> I["UTF-8 to stdout"]
    G -->|yes| J{"Ghostty or Kitty?"}
    J -->|no| K["Bail unsupported terminal"]
    J -->|yes| L["TerminalViewer::run"]
  end
```

Headless mode resolves images/Mermaid where possible but **never** opens the alternate screen or Kitty graphics.

## 2. Interactive viewer: two render modes

On startup, `TerminalViewer` calls `initial_render_state` ([`src/ui/terminal.rs`](../src/ui/terminal.rs)): it **tries “graphic page” mode first** (GitHub-like HTML → rasterize → slice into rows). If that pipeline fails (e.g. WebKit snapshot unavailable), it **falls back** to the structured TUI: `render_document` builds per-line layout, Syntect-highlighted code, and per-block Kitty images.

```mermaid
flowchart TB
  subgraph Init["TerminalViewer::new"]
    A["Document plus source text"] --> B["initial_render_state"]
  end

  B --> C{"render_graphic_page OK?"}
  C -->|yes| D["build_github_html"]
  D --> E["render_html_to_png WebKit"]
  E --> F["build_graphic_page"]
  F --> G["GraphicPage only mode"]

  C -->|no| H["render_document"]
  H --> I["RenderedDocument TUI mode"]
  I --> J["Warning if graphic failed"]

  subgraph Loop["Event loop"]
    K["draw per frame"] --> L{"graphic_page?"}
    L -->|Some| M["Kitty strips from full-page PNG"]
    L -->|None| N["draw_line and collect_graphics_commands"]
    N --> O["Lazy Mermaid rasterize"]
  end

  G --> Loop
  J --> Loop
```

`--watch` re-reads the file, re-parses with `parse_document`, and runs the same `initial_render_state` path again.

## 3. One frame: text grid + graphics protocol

```mermaid
flowchart LR
  subgraph Text["Text layer"]
    A["Clear content rows"] --> B["crossterm Print per RenderedLine"]
    B --> C["Syntect for code fences"]
  end

  subgraph Gfx["Graphics layer"]
    D["Delete stale placements if needed"] --> E["Kitty a=t transmit and a=p place"]
  end

  subgraph Chrome["Chrome"]
    F["Status line metadata"]
  end

  Text --> Gfx
  Gfx --> F
```

Kitty / Ghostty receive ANSI escapes from **crossterm** for text and **custom escape sequences** (`io::kitty_graphics`) for images. On exit, the viewer leaves alternate screen and sends a delete-all-placements command so the shell is left clean.

In **GraphicPage** mode the text loop has no `RenderedDocument` lines to paint; the viewport is driven almost entirely by the Kitty strip placements from the snapshot PNG.

## See also

- [`TECH.md`](./TECH.md) — deeper technical notes and constraints (internal doc, partly in Japanese).
