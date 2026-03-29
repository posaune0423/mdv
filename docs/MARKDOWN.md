# Markdown (GFM)

Authoring is meant to follow the **[GitHub Flavored Markdown Specification](https://github.github.com/gfm/)** (GFM). Parsing uses **[Comrak](https://github.com/kivikakk/comrak)**; enabled extensions mirror common GFM surface area:

| Extension | Notes |
|-----------|--------|
| **Strikethrough** | `~~text~~` |
| **Tables** | Pipe tables |
| **Autolink** | URLs and emails auto-linked where supported |
| **Task lists** | `- [ ]` / `- [x]` |
| **Footnotes** | Reference-style footnotes |
| **Alerts** | `> [!NOTE]`, `> [!TIP]`, etc. ([syntax](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts)); blockquotes starting with `[!NOTE]`–style tags are also treated as callouts |
| **Math** | GitHub-style `$...$` and `$$...$$` expressions via Comrak math extensions |

mdv also renders **fenced `mermaid` code blocks** (not part of GFM) when Mermaid is available. Raw HTML blocks are not rendered as HTML in the TUI; content is shown as plain text.

Exact edge-case behavior follows Comrak’s parser, not a formal proof of spec compliance—when in doubt, compare with the [GFM spec](https://github.github.com/gfm/) and Comrak’s release notes.

## Features at a glance

| Area | Details |
|------|---------|
| **Markdown** | GFM-oriented parsing via Comrak—headings, lists, tables, task lists, footnotes, alerts / callouts, links, images, thematic breaks, math |
| **Code** | Syntax highlighting powered by [Syntect](https://github.com/trishume/syntect) |
| **Assets** | PNG, JPEG, GIF, WebP, and SVG rendering where the terminal supports it |
| **Diagrams** | [Mermaid](https://mermaid.js.org/) diagrams (optional `--no-mermaid` to disable) |
| **Themes** | `system` (default), `light`, `dark` (`--theme`) |
| **Workflow** | `--watch` reloads when the file changes on disk |
| **CI / pipes** | Non-interactive use prints a plain-text rendering to stdout |
