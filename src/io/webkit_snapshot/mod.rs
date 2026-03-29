use std::path::Path;

#[cfg(target_os = "macos")]
use std::{
    fs,
    process::Command,
    sync::{Mutex, OnceLock},
};

#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::{Result, bail};
#[cfg(target_os = "macos")]
use tracing::info_span;

mod diagnostics;
#[cfg(any(target_os = "macos", test))]
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

#[cfg(not(target_os = "macos"))]
pub fn render_html_to_png(
    _html: &str,
    _base_dir: &Path,
    _viewport_width_px: u32,
) -> Result<SnapshotResult> {
    bail!("webkit snapshot rendering is only supported on macOS");
}

#[cfg(target_os = "macos")]
fn snapshot_render_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
