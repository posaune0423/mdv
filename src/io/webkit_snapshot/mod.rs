use std::path::Path;

#[cfg(target_os = "macos")]
use std::{
    fs,
    process::Command,
    sync::{Mutex, OnceLock},
};

#[cfg(target_os = "linux")]
use std::{fs, path::PathBuf, sync::Arc, thread, time::Duration};

#[cfg(any(target_os = "macos", target_os = "linux"))]
use anyhow::Context;
use anyhow::{Result, bail};
#[cfg(target_os = "linux")]
use headless_chrome::{Browser, LaunchOptions, protocol::cdp::Page, types::Bounds};
#[cfg(target_os = "macos")]
use tracing::info_span;

mod diagnostics;
#[cfg(any(target_os = "macos", target_os = "linux", test))]
mod paths;
mod script;
#[cfg(test)]
mod tests;

pub use self::diagnostics::{
    SnapshotAssetDiagnostics, SnapshotDiagnostics, SnapshotResult, SnapshotTypographyDiagnostics,
};
#[cfg(target_os = "macos")]
use self::{
    paths::{cleanup_workspace, common_read_access_root, create_workspace},
    script::SWIFT_SNAPSHOT_SCRIPT,
};
#[cfg(target_os = "linux")]
use self::paths::{cleanup_workspace, create_workspace};

#[cfg(target_os = "macos")]
pub fn render_html_to_png(
    html: &str,
    base_dir: &Path,
    viewport_width_px: u32,
) -> Result<SnapshotResult> {
    let _span = info_span!("webkit_snapshot.render_html_to_png", viewport_width_px).entered();
    let _guard = snapshot_render_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("snapshot render lock poisoned"))?;
    let read_access_root = common_read_access_root(html, base_dir);
    let workspace = create_workspace(base_dir, &read_access_root)?;
    let html_path = workspace.join("document.html");
    let script_path = workspace.join("snapshot.swift");
    let output_path = workspace.join("snapshot.png");
    let report_path = workspace.join("snapshot-report.json");

    fs::write(&html_path, html).context("failed to write html snapshot input")?;
    fs::write(&script_path, SWIFT_SNAPSHOT_SCRIPT)
        .context("failed to write swift snapshot helper")?;
    let output = Command::new("swift")
        .arg(&script_path)
        .arg(&html_path)
        .arg(&read_access_root)
        .arg(&output_path)
        .arg(&report_path)
        .arg(viewport_width_px.to_string())
        .output()
        .context("failed to execute swift snapshot helper")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let _ = cleanup_workspace(&workspace);
        if stderr.is_empty() {
            bail!("swift snapshot helper exited with {}", output.status);
        }
        bail!("swift snapshot helper exited with {}: {}", output.status, stderr);
    }

    let png_bytes = fs::read(&output_path).context("swift snapshot helper did not produce png")?;
    let diagnostics = fs::read(&report_path)
        .context("swift snapshot helper did not produce diagnostics report")
        .and_then(|bytes| {
            serde_json::from_slice::<SnapshotDiagnostics>(&bytes)
                .context("swift snapshot helper produced invalid diagnostics report")
        })?;
    let _ = cleanup_workspace(&workspace);
    Ok(SnapshotResult { png_bytes, diagnostics })
}

// --- Linux implementation using headless Chrome ---

/// Maximum number of visual stability probes before giving up.
#[cfg(target_os = "linux")]
const MAX_PROBES: u32 = 40;

/// Interval between stability probes.
#[cfg(target_os = "linux")]
const PROBE_INTERVAL: Duration = Duration::from_millis(25);

/// Height difference threshold (in pixels) to consider the layout stable.
#[cfg(target_os = "linux")]
const HEIGHT_TOLERANCE: f64 = 0.5;

/// Number of consecutive stable probes required before taking the screenshot.
#[cfg(target_os = "linux")]
const STABLE_COUNT_REQUIRED: u32 = 2;

/// JavaScript expression that probes the page's visual readiness.
///
/// Returns a JSON string with `height`, `fontsReady`, `imagesReady`, and
/// `mermaidsReady` fields.
#[cfg(target_os = "linux")]
const VISUAL_STABILITY_JS: &str = r#"
JSON.stringify((() => {
  const height = Math.max(
    document.documentElement.scrollHeight || 0,
    document.body.scrollHeight || 0,
    document.documentElement.offsetHeight || 0,
    document.body.offsetHeight || 0
  );
  const fontsReady = !document.fonts || document.fonts.status === "loaded";
  const imagesReady = Array.from(document.images || [])
    .filter(i => i.getAttribute("loading") !== "lazy")
    .every(i => i.complete && (i.naturalWidth > 0 || !i.currentSrc));
  const mermaids = Array.from(
    document.querySelectorAll(".mdv-mermaid-diagram")
  ).map((diagram) => {
    const rect = diagram.getBoundingClientRect();
    return { renderedWidth: rect.width || 0, renderedHeight: rect.height || 0 };
  });
  const mermaidsReady = mermaids.every(
    (d) => d.renderedWidth > 0 && d.renderedHeight > 0
  );
  return { height, fontsReady, imagesReady, mermaidsReady };
})())
"#;

#[cfg(target_os = "linux")]
pub fn render_html_to_png(
    html: &str,
    base_dir: &Path,
    viewport_width_px: u32,
) -> Result<SnapshotResult> {
    // Canonicalize base_dir so that relative paths like "." are resolved to
    // absolute paths before being passed to create_workspace and Chrome.
    let base_dir = fs::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
    let workspace = create_workspace(&base_dir, &base_dir)?;

    let result = render_html_to_png_inner(html, &workspace, viewport_width_px);

    let _ = cleanup_workspace(&workspace);
    result
}

#[cfg(target_os = "linux")]
fn render_html_to_png_inner(
    html: &str,
    workspace: &Path,
    viewport_width_px: u32,
) -> Result<SnapshotResult> {
    let html_path = workspace.join("document.html");
    fs::write(&html_path, html).context("failed to write html snapshot input")?;

    let file_url = url::Url::from_file_path(&html_path)
        .map_err(|()| anyhow::anyhow!("failed to convert workspace path to file URL: {}", html_path.display()))?
        .to_string();

    let chrome_path = find_chrome_binary()?;
    let initial_height: u32 = 900;

    let launch_options = LaunchOptions::default_builder()
        .path(Some(chrome_path))
        .window_size(Some((viewport_width_px, initial_height)))
        .idle_browser_timeout(Duration::from_secs(60))
        .args(vec![
            std::ffi::OsStr::new("--allow-file-access-from-files"),
            std::ffi::OsStr::new("--force-device-scale-factor=1"),
        ])
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build Chrome launch options: {e}"))?;

    let browser = Browser::new(launch_options).context("failed to launch headless Chrome")?;
    let tab = browser.new_tab().context("failed to create Chrome tab")?;

    tab.navigate_to(&file_url)
        .context("failed to navigate to HTML file")?;
    tab.wait_until_navigated()
        .context("Chrome tab failed to finish navigation")?;

    let probe = await_visual_stability(&tab)?;
    // Cap height to prevent OOM from pathological HTML inputs.
    const MAX_SNAPSHOT_HEIGHT: f64 = 32_000.0;
    let snapshot_height = probe.height.ceil().clamp(1.0, MAX_SNAPSHOT_HEIGHT);

    tab.set_bounds(Bounds::Normal {
        left: None,
        top: None,
        width: Some(f64::from(viewport_width_px)),
        height: Some(snapshot_height),
    })
    .context("failed to resize Chrome viewport")?;

    // Brief pause to let Chrome repaint after resize.
    thread::sleep(Duration::from_millis(20));

    let png_bytes = tab
        .capture_screenshot(
            Page::CaptureScreenshotFormatOption::Png,
            None,
            None,
            true,
        )
        .context("failed to capture PNG screenshot")?;

    Ok(SnapshotResult {
        png_bytes,
        diagnostics: SnapshotDiagnostics {
            fonts_ready: probe.fonts_ready,
            images_ready: probe.images_ready,
            mermaids_ready: probe.mermaids_ready,
            ..SnapshotDiagnostics::default()
        },
    })
}

/// Locate a Chrome / Chromium binary.
///
/// Checks the `MDV_CHROME_PATH` environment variable first, then falls back to
/// the default search performed by `headless_chrome`.
#[cfg(target_os = "linux")]
fn find_chrome_binary() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("MDV_CHROME_PATH") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
        bail!(
            "MDV_CHROME_PATH is set to {path:?} but the file does not exist"
        );
    }
    headless_chrome::browser::default_executable()
        .map_err(|e| anyhow::anyhow!("could not find Chrome/Chromium binary: {e}"))
}

/// Result of visual stability probing.
#[cfg(target_os = "linux")]
struct StabilityProbe {
    height: f64,
    fonts_ready: bool,
    images_ready: bool,
    mermaids_ready: bool,
}

/// Poll the page until fonts, images, and mermaid diagrams are ready **and**
/// the measured content height has stabilised across consecutive probes.
#[cfg(target_os = "linux")]
fn await_visual_stability(tab: &Arc<headless_chrome::Tab>) -> Result<StabilityProbe> {
    let mut last_height: f64 = -1.0;
    let mut stable_count: u32 = 0;
    let mut any_probe_succeeded = false;

    for probe in 0..=MAX_PROBES {
        // If any step of the probe fails (JS evaluation, JSON parsing), treat
        // the page as "not ready" and retry on the next iteration rather than
        // aborting the entire render.
        let probe_result: Result<serde_json::Value> = (|| {
            let remote = tab
                .evaluate(VISUAL_STABILITY_JS, false)
                .context("JS evaluation failed")?;
            let json_str = remote
                .value
                .as_ref()
                .and_then(|v| v.as_str())
                .context("probe returned no value")?;
            Ok(serde_json::from_str(json_str).context("failed to parse JSON")?)
        })();

        let payload = match probe_result {
            Ok(v) => v,
            Err(_) => {
                thread::sleep(PROBE_INTERVAL);
                continue;
            }
        };

        any_probe_succeeded = true;

        let height = payload
            .get("height")
            .and_then(|v| v.as_f64())
            .unwrap_or(900.0);
        let fonts_ready = payload
            .get("fontsReady")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let images_ready = payload
            .get("imagesReady")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mermaids_ready = payload
            .get("mermaidsReady")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let visuals_ready = fonts_ready && images_ready && mermaids_ready;

        if visuals_ready && (last_height - height).abs() < HEIGHT_TOLERANCE {
            stable_count += 1;
        } else {
            stable_count = 0;
        }
        last_height = height;

        if (visuals_ready && stable_count >= STABLE_COUNT_REQUIRED) || probe >= MAX_PROBES {
            return Ok(StabilityProbe {
                height,
                fonts_ready,
                images_ready,
                mermaids_ready,
            });
        }

        thread::sleep(PROBE_INTERVAL);
    }

    // If every single probe failed, we have no usable height measurement.
    if !any_probe_succeeded {
        bail!("all visual stability probes failed; Chrome may have crashed or the page is invalid");
    }

    // Unreachable when any probe succeeded (the loop returns Ok above), but
    // keep the compiler happy.
    Ok(StabilityProbe {
        height: last_height,
        fonts_ready: false,
        images_ready: false,
        mermaids_ready: false,
    })
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn render_html_to_png(
    _html: &str,
    _base_dir: &Path,
    _viewport_width_px: u32,
) -> Result<SnapshotResult> {
    bail!("snapshot rendering is not yet supported on this platform");
}

#[cfg(target_os = "macos")]
fn snapshot_render_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
