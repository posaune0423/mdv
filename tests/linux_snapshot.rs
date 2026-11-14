#![cfg(target_os = "linux")]

#[test]
fn linux_dispatches_html_to_a_png_snapshot() -> anyhow::Result<()> {
    let base_dir = tempfile::tempdir()?;
    let raster =
        mdv::io::snapshot::render_html_to_raster("<h1>Linux snapshot</h1>", base_dir.path(), 320)?;
    assert_eq!(image::guess_format(&raster)?, image::ImageFormat::Png);
    Ok(())
}
