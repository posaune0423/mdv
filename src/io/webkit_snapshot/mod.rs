use std::path::Path;

#[cfg(target_os = "macos")]
use std::{
    fs,
    sync::{Mutex, OnceLock},
};

#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::Result;
#[cfg(not(target_os = "macos"))]
use anyhow::bail;
#[cfg(target_os = "macos")]
use tracing::info_span;

mod diagnostics;
#[cfg(any(target_os = "macos", test))]
mod paths;
#[cfg(test)]
mod tests;
#[cfg(target_os = "macos")]
mod worker;

pub use self::diagnostics::{
    SnapshotAssetDiagnostics, SnapshotDiagnostics, SnapshotResult, SnapshotTypographyDiagnostics,
};
#[cfg(target_os = "macos")]
use self::paths::{cleanup_workspace, common_read_access_root, create_workspace};

#[cfg(target_os = "macos")]
const SNAPSHOT_HELPER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mdv-snapshot-helper"));

pub fn render_html_to_png(
    html: &str,
    base_dir: &Path,
    viewport_width_px: u32,
) -> Result<SnapshotResult> {
    let (png_bytes, diagnostics) = render_image(html, base_dir, viewport_width_px, "png")?;
    Ok(SnapshotResult { png_bytes, diagnostics })
}

pub(crate) fn render_html_to_tiff(html: &str, base_dir: &Path, width: u32) -> Result<Vec<u8>> {
    render_image(html, base_dir, width, "tiff").map(|(bytes, _)| bytes)
}

#[cfg(target_os = "macos")]
fn render_image(
    html: &str,
    base_dir: &Path,
    viewport_width_px: u32,
    format: &str,
) -> Result<(Vec<u8>, SnapshotDiagnostics)> {
    let _span = info_span!("webkit_snapshot.render_image", viewport_width_px, format).entered();
    let _guard = snapshot_render_lock()
        .lock()
        .map_err(|_| anyhow::anyhow!("snapshot render lock poisoned"))?;
    let read_access_root = common_read_access_root(html, base_dir);
    let workspace = create_workspace(base_dir, &read_access_root)?;
    let result =
        render_in_workspace(html, &read_access_root, &workspace, viewport_width_px, format);
    let _ = cleanup_workspace(&workspace);
    result
}

#[cfg(target_os = "macos")]
fn render_in_workspace(
    html: &str,
    read_access_root: &Path,
    workspace: &Path,
    viewport_width_px: u32,
    format: &str,
) -> Result<(Vec<u8>, SnapshotDiagnostics)> {
    let html_path = workspace.join("document.html");
    let output_path = workspace.join(format!("snapshot.{format}"));
    let report_path = workspace.join("snapshot-report.json");

    fs::write(&html_path, html).context("failed to write html snapshot input")?;
    worker::render(
        &html_path,
        read_access_root,
        &output_path,
        &report_path,
        viewport_width_px,
        format,
    )?;

    let bytes = fs::read(&output_path).context("snapshot helper did not produce raster")?;
    let diagnostics = fs::read(&report_path)
        .context("snapshot helper did not produce diagnostics report")
        .and_then(|bytes| {
            serde_json::from_slice::<SnapshotDiagnostics>(&bytes)
                .context("snapshot helper produced invalid diagnostics report")
        })?;
    Ok((bytes, diagnostics))
}

#[cfg(not(target_os = "macos"))]
fn render_image(
    _html: &str,
    _base_dir: &Path,
    _viewport_width_px: u32,
    _format: &str,
) -> Result<(Vec<u8>, SnapshotDiagnostics)> {
    bail!("webkit snapshot rendering is only supported on macOS");
}

#[cfg(target_os = "macos")]
fn snapshot_render_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
