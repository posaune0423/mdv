# Linux `render_html_to_png` Implementation

## Summary

Replace the macOS-only `render_html_to_png` stub on Linux with a working implementation using `headless_chrome` crate, enabling Graphic Mode (Kitty Graphics Protocol) on Linux terminals.

## Context

mdv's Graphic Mode pipeline:

```
Markdown → build_github_html() → HTML string → render_html_to_png() → PNG bytes → Kitty Graphics
```

All stages are cross-platform except `render_html_to_png()`, which currently uses a Swift + WebKit process on macOS and `bail!()` on all other platforms.

## Approach

Use `headless_chrome` (sync Rust crate wrapping Chrome DevTools Protocol) to load the HTML and capture a full-page PNG screenshot, with the same visual stability probing as the macOS Swift implementation.

### Why `headless_chrome`

- Synchronous API matches the existing codebase (no async runtime needed)
- Lightweight dependencies compared to alternatives (e.g., `chromiumoxide`)
- Single screenshot use case doesn't need async concurrency
- Actively maintained (v1.0.21, Feb 2026)

## Architecture

### Chrome Binary Resolution

```
1. Check MDV_CHROME_PATH environment variable
2. Search PATH for: google-chrome, google-chrome-stable, chromium, chromium-browser
3. If not found: bail with "Chrome/Chromium not found. Install chromium or set MDV_CHROME_PATH"
```

No automatic download. Users must have Chrome/Chromium installed.

### Snapshot Flow (Linux)

```
1. Create workspace via existing create_workspace() (paths.rs)
2. Write HTML to workspace/document.html
3. Launch headless Chrome with:
   - --allow-file-access-from-files (local image access)
   - --disable-web-security (CORS for local files)
   - --disable-gpu (headless, no GPU needed)
4. Navigate to file:///workspace/document.html
5. Probe visual stability (see below)
6. Resize viewport to full content height
7. Capture full-page PNG screenshot
8. Cleanup workspace
9. Return SnapshotResult { png_bytes, diagnostics: Default::default() }
```

### Visual Stability Probing

Port the Swift version's probing logic to Rust + JavaScript evaluation:

**JS probe (evaluated via `tab.evaluate()`):**

```javascript
{
  height: Math.max(
    document.body.offsetHeight, document.body.scrollHeight,
    document.documentElement.offsetHeight, document.documentElement.scrollHeight
  ),
  fontsReady: document.fonts.status === "loaded",
  imagesReady: [...document.querySelectorAll("img")]
    .filter(i => i.getAttribute("loading") !== "lazy")
    .every(i => i.complete && i.naturalWidth > 0),
  mermaidsReady: [...document.querySelectorAll(".mdv-mermaid-diagram svg")]
    .every(s => s.getBoundingClientRect().width > 0)
}
```

**Stability detection constants (matching Swift):**

| Constant | Value |
|----------|-------|
| `MAX_PROBES` | 40 |
| `PROBE_INTERVAL` | 25ms |
| `HEIGHT_TOLERANCE` | 0.5px |
| `STABLE_COUNT_REQUIRED` | 2 |

**Logic:**
- Repeat probe up to 40 times (max ~1 second)
- `visuals_ready = fonts_ready && images_ready && mermaids_ready`
- If visuals ready and height stable (within 0.5px) for 2 consecutive probes: take screenshot
- After 40 attempts: take screenshot anyway (same as Swift timeout behavior)

### Diagnostics

`SnapshotDiagnostics::default()` is returned. The diagnostics struct is not consumed by any caller — only `png_bytes` is used downstream in `build_graphic_page()`.

### Workspace Management

Reuse existing `paths.rs` by broadening its `#![cfg]` gate from `#![cfg(any(target_os = "macos", test))]` to `#![cfg(any(target_os = "macos", target_os = "linux", test))]`. This provides:
- Workspace creation under `base_dir/.mdv-webkit/<timestamp>/`
- Fallback to temp dir
- Cleanup after completion

## Files Changed

| File | Change |
|------|--------|
| `Cargo.toml` | Add `headless_chrome` as Linux-only dependency |
| `src/io/webkit_snapshot/mod.rs` | Add `#[cfg(target_os = "linux")]` block; change existing stub to `#[cfg(not(any(target_os = "macos", target_os = "linux")))]` |
| `src/io/webkit_snapshot/paths.rs` | Broaden `#![cfg]` gate to `any(target_os = "macos", target_os = "linux", test)` |
| `src/ui/terminal/mod.rs` | Add `bcon` to `is_supported_terminal()` |

**No new files.** No changes to macOS code paths.

### Cargo.toml Addition

```toml
[target.'cfg(target_os = "linux")'.dependencies]
headless_chrome = "1.0"
```

This keeps the dependency Linux-specific. Future platforms (e.g., Windows with WebView2) can add their own implementation and dependencies without conflict.

## Error Handling

All errors propagate via `anyhow::bail!` / `?` operator. The existing caller in `ui/terminal/layout.rs` already handles errors gracefully by falling back to text mode with a warning message.

| Scenario | Error message |
|----------|---------------|
| Chrome not found | `"Chrome/Chromium not found. Install chromium or set MDV_CHROME_PATH"` |
| Chrome launch failure | `"failed to launch Chrome: {detail}"` |
| Navigation failure | `"failed to load HTML: {detail}"` |
| Screenshot failure | `"failed to capture screenshot: {detail}"` |
| Probe timeout | Not an error — screenshot taken at timeout (matches Swift behavior) |

## Out of Scope

- Automatic Chrome download (users install via package manager)
- `SnapshotDiagnostics` population (unused by callers)
- Changes to Swift/macOS code path
- `chromiumoxide` or async runtime introduction
- Shared probe JS between Swift and Rust (future improvement)
