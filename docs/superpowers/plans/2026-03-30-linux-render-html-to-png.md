# Linux `render_html_to_png` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable Graphic Mode on Linux by implementing `render_html_to_png` using headless Chrome.

**Architecture:** Replace the `bail!()` stub in `src/io/webkit_snapshot/mod.rs` with a `#[cfg(target_os = "linux")]` implementation that uses the `headless_chrome` crate. Chrome loads the HTML, JavaScript probes visual stability (fonts, images, Mermaid), then a full-page PNG screenshot is captured.

**Tech Stack:** Rust, `headless_chrome` 1.0, Chrome/Chromium (system-installed)

**Spec:** `docs/superpowers/specs/2026-03-30-linux-render-html-to-png-design.md`

**Lint rules:** `unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`, `todo = "deny"`, `dbg_macro = "deny"`. All error handling via `?` operator or `unwrap_or_else(|e| panic!(...))` in tests only.

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` | Modify | Add `headless_chrome` as Linux-only dependency |
| `src/io/webkit_snapshot/paths.rs:1` | Modify | Broaden `#![cfg]` gate to include Linux |
| `src/io/webkit_snapshot/mod.rs` | Modify | Add Linux implementation, update fallback `#[cfg]` |
| `src/ui/terminal/mod.rs:85-88` | Modify | Add `bcon` to `is_supported_terminal()` |

---

### Task 1: Add `headless_chrome` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add Linux-only dependency to Cargo.toml**

Add at the end of the file:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
headless_chrome = "1.0"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors. `headless_chrome` is downloaded and compiled.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add headless_chrome dependency for Linux snapshot support"
```

---

### Task 2: Broaden `paths.rs` cfg gate

**Files:**
- Modify: `src/io/webkit_snapshot/paths.rs:1`

- [ ] **Step 1: Update the cfg gate**

Change line 1 from:

```rust
#![cfg(any(target_os = "macos", test))]
```

to:

```rust
#![cfg(any(target_os = "macos", target_os = "linux", test))]
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors. The `paths` module is now available on Linux.

- [ ] **Step 3: Verify existing tests still pass**

Run: `cargo test -- webkit_snapshot`
Expected: All existing `webkit_snapshot` tests pass (they use `#[cfg(test)]` which is already included in the gate).

- [ ] **Step 4: Commit**

```bash
git add src/io/webkit_snapshot/paths.rs
git commit -m "feat: enable webkit_snapshot paths module on Linux"
```

---

### Task 3: Implement Linux `render_html_to_png`

**Files:**
- Modify: `src/io/webkit_snapshot/mod.rs`

- [ ] **Step 1: Add Linux imports and Chrome binary resolver**

Add these imports and the `find_chrome_binary` function. Place them after the existing macOS imports (after line 14), guarded by `#[cfg(target_os = "linux")]`:

```rust
#[cfg(target_os = "linux")]
use std::fs;

#[cfg(target_os = "linux")]
use anyhow::Context;

#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use self::paths::{cleanup_workspace, create_workspace};

#[cfg(target_os = "linux")]
fn find_chrome_binary() -> Result<PathBuf> {
    // Check MDV_CHROME_PATH first
    if let Ok(path) = std::env::var("MDV_CHROME_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        bail!("MDV_CHROME_PATH is set to '{}' but the file does not exist", path.display());
    }

    // Fall back to headless_chrome's built-in discovery
    // (checks CHROME env var, then searches PATH for google-chrome-stable, chromium, etc.)
    headless_chrome::browser::default_executable()
        .map_err(|e| anyhow::anyhow!(
            "Chrome/Chromium not found. Install chromium or set MDV_CHROME_PATH. Details: {e}"
        ))
}
```

- [ ] **Step 2: Add the JS probe constant and probe result struct**

Add after `find_chrome_binary`, still inside the `#[cfg(target_os = "linux")]` guard:

```rust
#[cfg(target_os = "linux")]
const PROBE_JS: &str = r#"(() => ({
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
}))()"#;

#[cfg(target_os = "linux")]
const MAX_PROBES: u32 = 40;
#[cfg(target_os = "linux")]
const PROBE_INTERVAL: std::time::Duration = std::time::Duration::from_millis(25);
#[cfg(target_os = "linux")]
const HEIGHT_TOLERANCE: f64 = 0.5;
#[cfg(target_os = "linux")]
const STABLE_COUNT_REQUIRED: u32 = 2;
```

- [ ] **Step 3: Implement the Linux `render_html_to_png`**

Replace the existing `#[cfg(not(target_os = "macos"))]` block (lines 82-89) with two new blocks:

```rust
#[cfg(target_os = "linux")]
pub fn render_html_to_png(
    html: &str,
    base_dir: &Path,
    viewport_width_px: u32,
) -> Result<SnapshotResult> {
    use headless_chrome::browser::tab::Tab;
    use headless_chrome::protocol::cdp::Page;
    use std::ffi::OsStr;

    let chrome_path = find_chrome_binary()?;

    let workspace = create_workspace(base_dir, base_dir)?;
    let html_path = workspace.join("document.html");
    fs::write(&html_path, html).context("failed to write html to workspace")?;

    let file_url = format!("file://{}", html_path.display());

    let launch_result = (|| -> Result<Vec<u8>> {
        let options = headless_chrome::LaunchOptions::default_builder()
            .path(Some(chrome_path))
            .window_size(Some((viewport_width_px, 900)))
            .args(vec![
                OsStr::new("--allow-file-access-from-files"),
                OsStr::new("--disable-web-security"),
            ])
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build launch options: {e}"))?;

        let browser = headless_chrome::Browser::new(options)
            .context("failed to launch Chrome")?;

        let tab = browser.new_tab().context("failed to create tab")?;
        tab.navigate_to(&file_url).context("failed to load HTML")?;
        tab.wait_until_navigated().context("failed waiting for navigation")?;

        let content_height = await_visual_stability(&tab)?;

        // Resize viewport to full content height for full-page capture
        tab.set_bounds(headless_chrome::browser::tab::Bounds::Normal {
            left: Some(0),
            top: Some(0),
            width: Some(f64::from(viewport_width_px)),
            height: Some(content_height),
        })
        .context("failed to resize viewport")?;

        // Brief pause for resize to take effect
        std::thread::sleep(std::time::Duration::from_millis(50));

        let png_bytes = tab
            .capture_screenshot(Page::CaptureScreenshotFormatOption::Png, None, None, true)
            .context("failed to capture screenshot")?;

        Ok(png_bytes)
    })();

    let _ = cleanup_workspace(&workspace);

    let png_bytes = launch_result?;

    Ok(SnapshotResult {
        png_bytes,
        diagnostics: SnapshotDiagnostics::default(),
    })
}

#[cfg(target_os = "linux")]
fn await_visual_stability(tab: &headless_chrome::browser::tab::Tab) -> Result<f64> {
    let mut last_height: f64 = 0.0;
    let mut stable_count: u32 = 0;

    for _attempt in 0..MAX_PROBES {
        let result = tab.evaluate(PROBE_JS, false);
        let probe = match result {
            Ok(remote_object) => remote_object.value.unwrap_or_default(),
            Err(_) => {
                std::thread::sleep(PROBE_INTERVAL);
                continue;
            }
        };

        let height = probe.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let fonts_ready = probe.get("fontsReady").and_then(|v| v.as_bool()).unwrap_or(false);
        let images_ready = probe.get("imagesReady").and_then(|v| v.as_bool()).unwrap_or(false);
        let mermaids_ready = probe.get("mermaidsReady").and_then(|v| v.as_bool()).unwrap_or(false);

        let visuals_ready = fonts_ready && images_ready && mermaids_ready;

        if visuals_ready && (height - last_height).abs() < HEIGHT_TOLERANCE {
            stable_count += 1;
        } else {
            stable_count = 0;
        }

        if stable_count >= STABLE_COUNT_REQUIRED {
            return Ok(height);
        }

        last_height = height;
        std::thread::sleep(PROBE_INTERVAL);
    }

    // Timeout: return last measured height (same as Swift behavior)
    Ok(last_height.max(1.0))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn render_html_to_png(
    _html: &str,
    _base_dir: &Path,
    _viewport_width_px: u32,
) -> Result<SnapshotResult> {
    bail!("snapshot rendering is not yet supported on this platform");
}
```

- [ ] **Step 4: Update mod.rs imports for Linux**

The existing `paths` import on line 28 is macOS-only. Add Linux access. In the existing `#[cfg(target_os = "macos")]` use block (lines 26-30), the `paths` module is already imported. The Linux block added in Step 1 imports `paths::{cleanup_workspace, create_workspace}` separately.

No additional import changes needed — the `#[cfg(target_os = "linux")]` use statements in Step 1 handle this.

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: Compiles without errors on Linux.

- [ ] **Step 6: Verify existing tests still pass**

Run: `cargo test`
Expected: All tests pass. The new code is behind `#[cfg(target_os = "linux")]` and doesn't affect test builds (which don't set target_os).

- [ ] **Step 7: Commit**

```bash
git add src/io/webkit_snapshot/mod.rs
git commit -m "feat: implement render_html_to_png for Linux using headless Chrome"
```

---

### Task 4: Add `bcon` to supported terminals

**Files:**
- Modify: `src/ui/terminal/mod.rs:85-88`

- [ ] **Step 1: Write the failing test**

There are no existing tests for `is_supported_terminal`. Add one at the end of the test module (or create a new `#[cfg(test)]` block near the function if none exists). Find the appropriate test location:

```rust
#[cfg(test)]
mod supported_terminal_tests {
    use super::is_supported_terminal;

    #[test]
    fn bcon_is_supported() {
        assert!(is_supported_terminal(Some("bcon"), None));
    }

    #[test]
    fn ghostty_is_supported() {
        assert!(is_supported_terminal(Some("ghostty"), None));
        assert!(is_supported_terminal(Some("Ghostty"), None));
    }

    #[test]
    fn kitty_is_supported() {
        assert!(is_supported_terminal(None, Some("xterm-kitty")));
    }

    #[test]
    fn unknown_terminal_is_not_supported() {
        assert!(!is_supported_terminal(Some("alacritty"), None));
        assert!(!is_supported_terminal(None, Some("xterm-256color")));
        assert!(!is_supported_terminal(None, None));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test supported_terminal_tests::bcon_is_supported`
Expected: FAIL — `bcon` is not recognized.

- [ ] **Step 3: Update `is_supported_terminal`**

Change lines 85-88 from:

```rust
pub fn is_supported_terminal(term_program: Option<&str>, term: Option<&str>) -> bool {
    term_program.is_some_and(|value| value.eq_ignore_ascii_case("ghostty"))
        || term.is_some_and(|value| value.contains("kitty"))
}
```

to:

```rust
pub fn is_supported_terminal(term_program: Option<&str>, term: Option<&str>) -> bool {
    term_program.is_some_and(|value| {
        value.eq_ignore_ascii_case("ghostty") || value.eq_ignore_ascii_case("bcon")
    }) || term.is_some_and(|value| value.contains("kitty"))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test supported_terminal_tests`
Expected: All 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/terminal/mod.rs
git commit -m "feat: add bcon to supported terminal list"
```

---

### Task 5: Manual integration test

This task cannot be automated — it requires a running terminal with Kitty Graphics support and Chrome installed.

- [ ] **Step 1: Verify Chrome is available**

Run: `which chromium || which google-chrome`
Expected: Path to Chrome binary is printed.

- [ ] **Step 2: Build mdv**

Run: `cargo build`
Expected: Build succeeds.

- [ ] **Step 3: Test with a sample Markdown file**

Run (in a Kitty Graphics-capable terminal like bcon, Ghostty, or Kitty):

```bash
TERM_PROGRAM=bcon cargo run -- examples/README.md
```

Expected: mdv opens in Graphic Mode — the Markdown is rendered as a styled image (GitHub-style), not just plain text. Scrolling works.

- [ ] **Step 4: Test error path — no Chrome**

Run:

```bash
MDV_CHROME_PATH=/nonexistent cargo run -- examples/README.md
```

Expected: Falls back to text mode with warning: `"graphic mode unavailable: MDV_CHROME_PATH is set to '/nonexistent' but the file does not exist"`

- [ ] **Step 5: Test with MDV_CHROME_PATH**

Run:

```bash
MDV_CHROME_PATH=$(which chromium) cargo run -- examples/README.md
```

Expected: Graphic Mode works using the specified Chrome binary.

---

## Summary

| Task | Description | Estimated Steps |
|------|-------------|----------------|
| 1 | Add `headless_chrome` dependency | 3 |
| 2 | Broaden `paths.rs` cfg gate | 4 |
| 3 | Implement Linux `render_html_to_png` | 7 |
| 4 | Add `bcon` to supported terminals | 5 |
| 5 | Manual integration test | 5 |
