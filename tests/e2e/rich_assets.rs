use assert_cmd::Command;
use image::{ImageBuffer, Rgba};
use predicates::str::contains;
use tempfile::TempDir;

#[test]
fn headless_render_resolves_local_image_and_mermaid_when_available() {
    let dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(error) => panic!("temp dir should be created: {error}"),
    };
    let image_path = dir.path().join("pixel.png");
    let image = ImageBuffer::from_pixel(1, 1, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    if let Err(error) = image.save(&image_path) {
        panic!("image fixture should be written: {error}");
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
        image_path.display()
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

    let doc_path = dir.path().join("doc.md");
    let source =
        format!("![pixel]({})\n\n```mermaid\ngraph TD\n    A --> B\n```\n", image_path.display());
    if let Err(error) = std::fs::write(&doc_path, source) {
        panic!("doc should be written: {error}");
    }

    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg(&doc_path)
        .env("MDV_MERMAID_CMD", &script)
        .assert()
        .success()
        .stdout(contains("[Image: pixel 1x1"))
        .stdout(contains("[Mermaid rendered: 1x1]"));
}
