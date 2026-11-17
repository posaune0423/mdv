use std::path::Path;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};

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
    let (png_bytes, diagnostics) =
        render_image(html, base_dir, viewport_width_px, "png", true, None)?;
    Ok(SnapshotResult { png_bytes, diagnostics })
}

#[cfg(target_os = "macos")]
pub(crate) fn render_html_to_tiff(
    html: &str,
    base_dir: &Path,
    width: u32,
    wait_for_remote: bool,
    cancelled: Option<&AtomicBool>,
) -> Result<(Vec<u8>, bool)> {
    let (bytes, diagnostics) =
        render_image(html, base_dir, width, "tiff", wait_for_remote, cancelled)?;
    let pending = diagnostics.images.iter().any(|image| {
        !image.complete
            && [&image.source, &image.current_src].iter().any(|source| {
                url::Url::parse(source).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
            })
    });
    Ok((bytes, pending))
}

#[cfg(target_os = "macos")]
fn render_image(
    html: &str,
    base_dir: &Path,
    viewport_width_px: u32,
    format: &str,
    wait_for_remote: bool,
    cancelled: Option<&AtomicBool>,
) -> Result<(Vec<u8>, SnapshotDiagnostics)> {
    let _span = info_span!("webkit_snapshot.render_image", viewport_width_px, format).entered();
    let lock = snapshot_render_lock(wait_for_remote);
    let _guard = loop {
        if cancelled.is_some_and(|token| token.load(Ordering::Acquire)) {
            anyhow::bail!("snapshot rendering cancelled");
        }
        match lock.try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::Poisoned(_)) => {
                anyhow::bail!("snapshot render lock poisoned")
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
    };
    if cancelled.is_some_and(|token| token.load(Ordering::Acquire)) {
        anyhow::bail!("snapshot rendering cancelled");
    }
    let read_access_root = common_read_access_root(html, base_dir);
    let workspace = create_workspace(base_dir, &read_access_root)?;
    let result = render_in_workspace(
        html,
        &read_access_root,
        &workspace,
        viewport_width_px,
        format,
        wait_for_remote,
        cancelled,
    );
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
    wait_for_remote: bool,
    cancelled: Option<&AtomicBool>,
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
        worker::RenderOptions { width: viewport_width_px, format, wait_for_remote, cancelled },
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
    _wait_for_remote: bool,
    _cancelled: Option<&std::sync::atomic::AtomicBool>,
) -> Result<(Vec<u8>, SnapshotDiagnostics)> {
    bail!("webkit snapshot rendering is only supported on macOS");
}

#[cfg(target_os = "macos")]
fn snapshot_render_lock(wait_for_remote: bool) -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    static REMOTE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let lock = if wait_for_remote { &REMOTE_LOCK } else { &LOCK };
    lock.get_or_init(|| Mutex::new(()))
}
