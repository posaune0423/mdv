use image::{ImageBuffer, Rgba};
use mdv::cli::Theme;
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
    -i|-o|-b|-w|-H|-s|-c|-C|-p)
      [ "$1" = "-o" ] && out="$2"
      shift 2
      ;;
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
    let output = match renderer.render_png("graph TD\n    A --> B\n", Theme::Light) {
        Ok(output) => output,
        Err(error) => panic!("render should succeed: {error}"),
    };

    assert!(!output.is_empty());
}

#[test]
fn passes_requested_width_and_scale_to_mermaid_cli() {
    let dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(error) => panic!("temp dir should be created: {error}"),
    };
    let png_path = dir.path().join("pixel.png");
    let args_path = dir.path().join("args.txt");
    let image = ImageBuffer::from_pixel(1, 1, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    if let Err(error) = image.save(&png_path) {
        panic!("png fixture should be written: {error}");
    }

    let script = dir.path().join("fake-mmdc.sh");
    let content = format!(
        r#"#!/bin/sh
set -eu
out=""
args_file="{}"
printf "%s\n" "$@" > "$args_file"
while [ "$#" -gt 0 ]; do
  case "$1" in
    -i|-o|-b|-w|-H|-s|-c|-C|-p)
      [ "$1" = "-o" ] && out="$2"
      shift 2
      ;;
    *) shift ;;
  esac
done
cp "{}" "$out"
"#,
        args_path.display(),
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
    let output = match renderer.render_png_sized(
        "graph TD\n    A --> B\n",
        Some(640),
        Some(2.0),
        Theme::Dark,
    ) {
        Ok(output) => output,
        Err(error) => panic!("render should succeed: {error}"),
    };

    assert!(!output.is_empty());
    let args = std::fs::read_to_string(args_path)
        .unwrap_or_else(|error| panic!("args should be captured: {error}"));
    assert!(args.contains("-w"));
    assert!(args.contains("640"));
    assert!(args.contains("-s"));
    assert!(args.contains("2"));
    assert!(args.contains("-t"));
    assert!(args.contains("dark"));
}

#[test]
fn surfaces_stderr_when_mermaid_cli_fails() {
    let dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(error) => panic!("temp dir should be created: {error}"),
    };
    let script = dir.path().join("fake-mmdc.sh");
    let content = r#"#!/bin/sh
set -eu
echo "parse error near node A" >&2
exit 7
"#;
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
    let error = match renderer.render_png("graph TD\n    A --> B\n", Theme::Light) {
        Ok(_) => panic!("render should fail"),
        Err(error) => error,
    };
    let message = error.to_string();

    assert!(message.contains("exit status: 7"));
    assert!(message.contains("parse error near node A"));
}

#[test]
fn reuses_cached_png_output_without_invoking_command_again() {
    let dir = TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let cache_dir =
        TempDir::new().unwrap_or_else(|error| panic!("cache dir should be created: {error}"));
    let png_path = dir.path().join("pixel.png");
    let counter_path = dir.path().join("counter.txt");
    let image = ImageBuffer::from_pixel(1, 1, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    image.save(&png_path).unwrap_or_else(|error| panic!("png fixture should be written: {error}"));

    let script = dir.path().join("fake-mmdc.sh");
    let content = format!(
        r#"#!/bin/sh
set -eu
out=""
counter_file="{}"
count=0
if [ -f "$counter_file" ]; then
  count=$(cat "$counter_file")
fi
count=$((count + 1))
printf "%s" "$count" > "$counter_file"
while [ "$#" -gt 0 ]; do
  case "$1" in
    -i|-o|-b|-w|-H|-s|-c|-C|-p)
      [ "$1" = "-o" ] && out="$2"
      shift 2
      ;;
    *) shift ;;
  esac
done
cp "{}" "$out"
"#,
        counter_path.display(),
        png_path.display()
    );
    std::fs::write(&script, content)
        .unwrap_or_else(|error| panic!("script should be written: {error}"));
    let status = std::process::Command::new("chmod").arg("+x").arg(&script).status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => panic!("chmod failed: {status}"),
        Err(error) => panic!("chmod failed to start: {error}"),
    }

    let renderer = MermaidCliRenderer::with_cache_dir(script, cache_dir.path().to_path_buf());

    renderer
        .render_png_sized("graph TD\n    A --> B\n", Some(640), Some(2.0), Theme::Light)
        .unwrap_or_else(|error| panic!("first render should succeed: {error}"));
    renderer
        .render_png_sized("graph TD\n    A --> B\n", Some(640), Some(2.0), Theme::Light)
        .unwrap_or_else(|error| panic!("second render should succeed: {error}"));

    let count = std::fs::read_to_string(counter_path)
        .unwrap_or_else(|error| panic!("counter should be readable: {error}"));
    assert_eq!(count, "1");
}

#[test]
fn caches_mermaid_output_per_theme() {
    let dir = TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let cache_dir =
        TempDir::new().unwrap_or_else(|error| panic!("cache dir should be created: {error}"));
    let png_path = dir.path().join("pixel.png");
    let counter_path = dir.path().join("counter.txt");
    let image = ImageBuffer::from_pixel(1, 1, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    image.save(&png_path).unwrap_or_else(|error| panic!("png fixture should be written: {error}"));

    let script = dir.path().join("fake-mmdc.sh");
    let content = format!(
        r#"#!/bin/sh
set -eu
out=""
counter_file="{}"
count=0
if [ -f "$counter_file" ]; then
  count=$(cat "$counter_file")
fi
count=$((count + 1))
printf "%s" "$count" > "$counter_file"
while [ "$#" -gt 0 ]; do
  case "$1" in
    -i|-o|-b|-w|-H|-s|-t|-c|-C|-p)
      [ "$1" = "-o" ] && out="$2"
      shift 2
      ;;
    *) shift ;;
  esac
done
cp "{}" "$out"
"#,
        counter_path.display(),
        png_path.display()
    );
    std::fs::write(&script, content)
        .unwrap_or_else(|error| panic!("script should be written: {error}"));
    let status = std::process::Command::new("chmod").arg("+x").arg(&script).status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => panic!("chmod failed: {status}"),
        Err(error) => panic!("chmod failed to start: {error}"),
    }

    let renderer = MermaidCliRenderer::with_cache_dir(script, cache_dir.path().to_path_buf());

    renderer
        .render_png_sized("graph TD\n    A --> B\n", Some(640), Some(2.0), Theme::Light)
        .unwrap_or_else(|error| panic!("light render should succeed: {error}"));
    renderer
        .render_png_sized("graph TD\n    A --> B\n", Some(640), Some(2.0), Theme::Dark)
        .unwrap_or_else(|error| panic!("dark render should succeed: {error}"));

    let count = std::fs::read_to_string(counter_path)
        .unwrap_or_else(|error| panic!("counter should be readable: {error}"));
    assert_eq!(count, "2");
}
