use image::{ImageBuffer, Rgba};
use mdv::io::mermaid_cli::MermaidCliRenderer;
use tempfile::TempDir;

#[test]
fn invokes_external_mermaid_command_and_reads_png_output() {
    let dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(error) => panic!("temp dir should be created: {error}"),
    };
    let png_path = dir.path().join("pixel.png");
    let image = ImageBuffer::from_pixel(1, 1, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    if let Err(error) = image.save(&png_path) {
        panic!("png fixture should be written: {error}");
    }

    let script = dir.path().join("fake-mmdc.sh");
    let content = format!(
        r#"#!/bin/sh
set -eu
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    *) shift ;;
  esac
done
cp "{}" "$out"
"#,
        png_path.display()
    );
    if let Err(error) = std::fs::write(&script, content) {
        panic!("script should be written: {error}");
    }
    let status = std::process::Command::new("chmod").arg("+x").arg(&script).status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => panic!("chmod failed: {status}"),
        Err(error) => panic!("chmod failed to start: {error}"),
    }

    let renderer = MermaidCliRenderer::new(script);
    let output = match renderer.render_png("graph TD\n    A --> B\n") {
        Ok(output) => output,
        Err(error) => panic!("render should succeed: {error}"),
    };

    assert!(!output.is_empty());
}
