# PRD: `mdv`

| Item | Value |
| --- | --- |
| Version | 1.0 |
| Status | Active |
| Product | `mdv` |
| Type | CLI Markdown Viewer written in Rust |
| Repository Version | `0.1.1` |
| Primary Interface | Terminal CLI |
| Document Basis | Repository audit on 2026-03-29 |

## 1. Purpose

`mdv` is a single-binary viewer for reading Markdown documents with high readability inside a terminal, without switching to a browser or editor preview.

The current product direction evident in this repository has two pillars:

- Bring a GitHub-like appearance to the terminal as faithfully as possible in interactive mode
- Return plain-text rendering that works reliably in pipes and CI in non-interactive mode

## 2. Problems to Solve

Frequent terminal users commonly encounter the following friction when reading Markdown:

- Opening a browser or IDE preview just to inspect a README, design document, or specification
- Reduced fidelity for tables, images, callouts, Mermaid, and syntax highlighting in existing terminal viewers
- The need for a minimally readable alternative when rich viewers are unavailable in CI or pipes

`mdv` aims to reduce this friction and bring Markdown reading back into the terminal workflow.

## 3. Target Users

### Primary User

- Software engineers who use Ghostty or Kitty daily
- People who read READMEs, design docs, and specifications in Markdown
- People who value workflows that stay in the terminal

### Secondary User

- Engineers who conduct reviews and research primarily through the CLI
- People who want readable Markdown formatting in CI or shell pipelines
- People who want local Markdown display similar to GitHub

## 4. Main Use Cases

| # | Use Case | Current Outcome |
| --- | --- | --- |
| 1 | Run `mdv README.md` to read a document | Graphic page display in an interactive TTY; plain-text display otherwise |
| 2 | Inspect illustrated design documents such as the rich fixture | Resolve local images and render them when display is available |
| 3 | Read specifications containing Mermaid | Render when a renderer is available; otherwise degrade to an unavailable indicator |
| 4 | Preview while editing | Reload saved changes automatically; `--watch=false` opts out |
| 5 | Read Markdown in a shell pipeline | Output headless formatting when stdout is not a TTY |

## 5. Current Product Scope

### 5.1 Input

- A local Markdown file path
- Primarily `.md` / `.markdown` files

### 5.2 CLI Options

- `--watch`
- `--theme system|light|dark`
- `--no-mermaid`
- `--version`
- `update`

### 5.3 Supported Markdown Elements

The implementation normalizes the following blocks into the `Document` intermediate representation:

- Heading
- Paragraph
- List
- BlockQuote
- Callout
- CodeFence
- Table
- Image
- Mermaid
- Rule
- Footnote

It also preserves the following inline elements:

- bold
- italic
- inline code
- link text and link destination

### 5.4 Interactive Experience

Interactive mode starts only when both `stdout` and `stdin` are TTYs and the terminal is identified as Ghostty or Kitty.

The current implementation supports these controls:

- `j` / `Down`: move down one line
- `k` / `Up`: move up one line
- `PageDown`
- `PageUp`
- `g`: go to the beginning
- `G`: go to the end
- `r`: reload
- `o`: open the first link in the document in the default browser
- `q`: quit

### 5.5 Non-Interactive Experience

Non-interactive mode uses the plain-text renderer and degrades rich elements as follows:

- Links use the `label <url>` format
- Images show metadata including dimensions when resolved, or a missing indicator on failure
- Mermaid is represented as rendered / disabled / unavailable
- Tables, footnotes, and callouts are formatted as readable text

## 6. Product Requirements

### 6.1 Functional Requirements

1. Parse a Markdown file once and normalize it into the internal `Document`
2. Start the terminal viewer in an interactive TTY
3. Return plain text instead of launching the viewer in non-interactive contexts
4. Resolve local images using relative paths
5. Keep the overall process running when no Mermaid renderer is available
6. Detect file content changes and reload automatically by default; `--watch=false` disables polling.
7. Default to `system`, with explicit `light` / `dark` overrides
8. Provide basic keyboard navigation
9. Use `update` to compare the current `mdv` executable against the tracked `bin/mdv` on GitHub `main`, overwriting the installation only when they differ

### 6.2 Non-Functional Requirements

1. Distribute as a single binary
2. Prohibit `unsafe` and maintain zero Clippy warnings
3. Pass `fmt`, `clippy -D warnings`, and `cargo test` in CI
4. Return at least an explanatory error or diagnostic when rich rendering fails

## 7. Non-Goals

Based on this repository, the following are currently outside the product scope or unimplemented:

- Markdown editing
- Search UI
- TOC panel
- Focus navigation between multiple links
- Official tmux / zellij support
- Remote image fetching
- Arbitrary HTML execution
- Windows support

## 8. Constraints Based on the Current Implementation

The README lists macOS / Linux as distribution targets, but current interactive rendering depends on converting GitHub HTML to PNG through WebKit. Because `src/io/webkit_snapshot` calls `bail!` on non-macOS platforms, the current rich interactive path effectively assumes macOS.

The `o` key opens `DocumentMeta.links.first()` rather than the currently focused link. Future UX wording and specifications need to either formalize this behavior or introduce actual focus navigation.

## 9. Success Metrics

The following short-term metrics are reasonable based on the codebase:

- Passing integration / e2e tests using rich fixtures
- Being able to read the README and documents under examples through to the end with `mdv`
- No silent failures for Mermaid or local images
- Maintaining a basic level of viewing quality in both interactive and non-interactive modes

## 10. Open Questions

- How to support the rich interactive path on Linux
- Whether to restore a runtime fallback when graphic page generation fails
- Whether to expand link navigation to multiple links
- How to prioritize viewer UX features such as search and a TOC
