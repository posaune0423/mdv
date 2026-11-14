#[path = "../support/fake_mermaid.rs"]
mod fake_mermaid;

#[cfg(target_os = "linux")]
use std::fs::OpenOptions;

use image::{ImageBuffer, Rgba};
use mdv::cli::Theme;
use mdv::io::mermaid_cli::MermaidCliRenderer;
use tempfile::TempDir;

fn png_fixture(directory: &TempDir) -> std::path::PathBuf {
    let png_path = directory.path().join("pixel.png");
    let image = ImageBuffer::from_pixel(1, 1, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    image.save(&png_path).unwrap_or_else(|error| panic!("png fixture should be written: {error}"));
    png_path
}

#[test]
fn invokes_external_mermaid_command_and_reads_png_output() {
    let directory =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let command =
        fake_mermaid::successful_command(directory.path(), &png_fixture(&directory), None, None)
            .unwrap_or_else(|error| panic!("fake Mermaid command should be created: {error}"));

    let renderer = MermaidCliRenderer::new(command);
    let output = renderer
        .render_png("graph TD\n    A --> B\n", Theme::Light)
        .unwrap_or_else(|error| panic!("render should succeed: {error:#}"));

    assert!(!output.is_empty());
}

#[test]
fn passes_requested_width_and_scale_to_mermaid_cli() {
    let directory =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let args_path = directory.path().join("args.txt");
    let command = fake_mermaid::successful_command(
        directory.path(),
        &png_fixture(&directory),
        Some(&args_path),
        None,
    )
    .unwrap_or_else(|error| panic!("fake Mermaid command should be created: {error}"));

    let renderer = MermaidCliRenderer::new(command);
    let output = renderer
        .render_png_sized("graph TD\n    A --> B\n", Some(640), Some(2.0), Theme::Dark)
        .unwrap_or_else(|error| panic!("render should succeed: {error:#}"));

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
    let directory =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let command = fake_mermaid::failing_command(directory.path())
        .unwrap_or_else(|error| panic!("failing Mermaid command should be created: {error}"));

    let renderer = MermaidCliRenderer::new(command);
    let error = match renderer.render_png("graph TD\n    A --> B\n", Theme::Light) {
        Ok(_) => panic!("render should fail"),
        Err(error) => error,
    };
    let message = format!("{error:#}");

    assert!(message.contains('7'));
    assert!(message.contains("parse error near node A"));
}

#[cfg(target_os = "linux")]
#[test]
fn runs_fake_mermaid_while_its_generated_content_is_open_for_writing() {
    let directory =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let command =
        fake_mermaid::successful_command(directory.path(), &png_fixture(&directory), None, None)
            .unwrap_or_else(|error| panic!("fake Mermaid command should be created: {error}"));
    let content = fake_mermaid::dynamic_content_path(directory.path());
    let _content_writer = OpenOptions::new()
        .write(true)
        .open(content)
        .unwrap_or_else(|error| panic!("fixture content should open for writing: {error}"));

    let output = MermaidCliRenderer::new(command)
        .render_png("graph TD\n    A --> B\n", Theme::Light)
        .unwrap_or_else(|error| panic!("fixture should execute while content is open: {error:#}"));

    assert!(!output.is_empty());
}

#[test]
fn reuses_cached_png_output_without_invoking_command_again() {
    let directory =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let cache_dir =
        TempDir::new().unwrap_or_else(|error| panic!("cache dir should be created: {error}"));
    let counter_path = directory.path().join("counter.txt");
    let command = fake_mermaid::successful_command(
        directory.path(),
        &png_fixture(&directory),
        None,
        Some(&counter_path),
    )
    .unwrap_or_else(|error| panic!("fake Mermaid command should be created: {error}"));

    let renderer = MermaidCliRenderer::with_cache_dir(command, cache_dir.path().to_path_buf());
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
    let directory =
        TempDir::new().unwrap_or_else(|error| panic!("temp dir should be created: {error}"));
    let cache_dir =
        TempDir::new().unwrap_or_else(|error| panic!("cache dir should be created: {error}"));
    let counter_path = directory.path().join("counter.txt");
    let command = fake_mermaid::successful_command(
        directory.path(),
        &png_fixture(&directory),
        None,
        Some(&counter_path),
    )
    .unwrap_or_else(|error| panic!("fake Mermaid command should be created: {error}"));

    let renderer = MermaidCliRenderer::with_cache_dir(command, cache_dir.path().to_path_buf());
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
