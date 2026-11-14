//! Platform-selected HTML rasterization for the interactive graphics pipeline.

use std::path::Path;

use anyhow::Result;
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
use anyhow::bail;

#[cfg(test)]
#[must_use]
pub(crate) fn has_graphic_snapshot_backend(target_os: &str) -> bool {
    matches!(target_os, "linux" | "macos" | "windows")
}

/// Renders an HTML document into the lossless raster consumed by `GraphicPage`.
pub fn render_html_to_raster(html: &str, base_dir: &Path, width: u32) -> Result<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        crate::io::webkit_snapshot::render_html_to_tiff(html, base_dir, width)
    }

    #[cfg(target_os = "windows")]
    {
        crate::io::chromium_snapshot::render_html_to_png(html, base_dir, width)
            .map(|result| result.png_bytes)
    }

    #[cfg(target_os = "linux")]
    {
        crate::io::chromium_snapshot::render_html_to_png(html, base_dir, width)
            .map(|result| result.png_bytes)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = (html, base_dir, width);
        bail!("interactive HTML rasterization is only supported on macOS, Windows, and Linux");
    }
}

#[cfg(test)]
mod tests {
    use super::has_graphic_snapshot_backend;

    #[test]
    fn linux_uses_the_chromium_graphic_snapshot_backend() {
        assert!(has_graphic_snapshot_backend("linux"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_routes_html_rasters_through_chromium() -> anyhow::Result<()> {
        let base_dir = tempfile::tempdir()?;
        let raster = super::render_html_to_raster("<h1>Linux snapshot</h1>", base_dir.path(), 320)?;
        assert_eq!(image::guess_format(&raster)?, image::ImageFormat::Png);
        Ok(())
    }
}
