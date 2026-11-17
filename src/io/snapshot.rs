//! Platform-selected HTML rasterization for the interactive graphics pipeline.

use std::{path::Path, sync::atomic::AtomicBool};

use anyhow::Result;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
use anyhow::bail;

/// Renders an HTML document into the lossless raster consumed by `GraphicPage`.
pub fn render_html_to_raster(html: &str, base_dir: &Path, width: u32) -> Result<Vec<u8>> {
    render_html_with_pending_assets(html, base_dir, width, true, None).map(|(bytes, _)| bytes)
}

/// Return an initial raster or wait for remote images on the background worker.
pub(crate) fn render_html_with_pending_assets(
    html: &str,
    base_dir: &Path,
    width: u32,
    wait_for_remote: bool,
    cancelled: Option<&AtomicBool>,
) -> Result<(Vec<u8>, bool)> {
    #[cfg(target_os = "macos")]
    {
        crate::io::webkit_snapshot::render_html_to_tiff(
            html,
            base_dir,
            width,
            wait_for_remote,
            cancelled,
        )
    }

    #[cfg(target_os = "windows")]
    {
        let _ = (wait_for_remote, cancelled);
        crate::io::chromium_snapshot::render_html_to_png(html, base_dir, width)
            .map(|result| (result.png_bytes, false))
    }

    #[cfg(target_os = "linux")]
    {
        let _ = (wait_for_remote, cancelled);
        crate::io::chromium_snapshot::render_html_to_png(html, base_dir, width)
            .map(|result| (result.png_bytes, false))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (html, base_dir, width, wait_for_remote, cancelled);
        bail!("interactive HTML rasterization is only supported on macOS, Windows, and Linux");
    }
}
