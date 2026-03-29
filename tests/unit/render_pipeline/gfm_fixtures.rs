use std::{fs, path::Path};

use mdv::{cli::Theme, core::config::MermaidMode, render::github_html::build_github_html};

#[cfg(target_os = "macos")]
use mdv::{
    io::webkit_snapshot::render_html_to_png,
    ui::page_graphics::{build_graphic_page, total_rows, viewport_slice},
};

fn fixture_dirs() -> Vec<std::path::PathBuf> {
    let root = Path::new("tests/fixtures/gfm");
    let mut dirs = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            path.is_dir().then_some(path)
        })
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn fixture_dir(name: &str) -> std::path::PathBuf {
    Path::new("tests/fixtures/gfm").join(name)
}

fn fixture_input(name: &str) -> String {
    fs::read_to_string(fixture_dir(name).join("input.md"))
        .unwrap_or_else(|error| panic!("failed to read fixture {name}: {error}"))
}

fn extract_article_body(html: &str) -> &str {
    let article_start = html
        .find("<article class=\"markdown-body entry-content container-lg\">")
        .unwrap_or_else(|| panic!("article wrapper missing"));
    let body_start = html[article_start..]
        .find('>')
        .map(|offset| article_start + offset + 1)
        .unwrap_or_else(|| panic!("article tag malformed"));
    let body_end = html[body_start..]
        .find("</article>")
        .map(|offset| body_start + offset)
        .unwrap_or_else(|| panic!("article closing tag missing"));
    html[body_start..body_end].trim()
}

#[test]
fn readme_style_img_tag_is_restored_in_github_html() {
    let source = "<div align=\"center\">\n\n# mdv\n\n<img src=\"docs/screenshot.jpg\" width=\"700\" alt=\"hero\" />\n\n</div>\n";
    let fixture_dir = fixture_dir("html-wrappers");
    let html = build_github_html(source, &fixture_dir, Theme::Dark, MermaidMode::Disabled)
        .unwrap_or_else(|error| panic!("should render: {error}"));
    let body = extract_article_body(&html);
    eprintln!("=== BODY ===\n{body}\n=== END ===");
    assert!(
        body.contains(r#"<img src="docs/screenshot.jpg""#),
        "img tag not found in body:\n{body}"
    );
}

#[test]
fn html_wrapper_fixture_preserves_raw_html_markers_in_github_html() {
    let fixture_dir = fixture_dir("html-wrappers");
    let html = build_github_html(
        &fixture_input("html-wrappers"),
        &fixture_dir,
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html wrapper fixture should render: {error}"));
    let body = extract_article_body(&html);

    assert!(body.contains(r#"<div align="center">"#));
    assert!(body.contains("<h1>Fixture Header</h1>"));
    assert!(body.contains("Paragraph with inline HTML<br/>kept as text."));
    assert!(body.contains("</div>"));
}

#[test]
fn html_img_fixture_restores_img_tags_in_github_html() {
    let fixture_dir = fixture_dir("html-img");
    let html = build_github_html(
        &fixture_input("html-img"),
        &fixture_dir,
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html-img fixture should render: {error}"));
    let body = extract_article_body(&html);

    assert!(
        body.contains(r#"<img src="../../../../examples/pixel.png""#),
        "standalone <img> should be restored, got:\n{body}"
    );
    assert!(
        body.contains(r#"width="700""#),
        "width attribute should be preserved, got:\n{body}"
    );
    assert!(
        body.contains(r#"<p align="center">"#),
        "<p align> should be restored, got:\n{body}"
    );
    assert!(
        body.contains("</p>"),
        "</p> should be restored, got:\n{body}"
    );
}

#[test]
fn badge_fixture_preserves_badge_images_in_github_html() {
    let fixture_dir = fixture_dir("badges-local");
    let html = build_github_html(
        &fixture_input("badges-local"),
        &fixture_dir,
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("badge fixture should render: {error}"));
    let body = extract_article_body(&html);

    assert!(body.contains(r#"<img src="badge-ci.svg" alt="CI""#));
    assert!(body.contains(r#"<img src="badge-license.svg" alt="License""#));
    assert!(body.contains(r#"href="https://example.com/ci""#));
    assert!(body.contains(r#"href="https://example.com/license""#));
}

#[test]
fn gfm_fixtures_generate_expected_html_fragments() {
    for fixture_dir in fixture_dirs() {
        let input = fs::read_to_string(fixture_dir.join("input.md"))
            .unwrap_or_else(|error| panic!("failed to read fixture input: {error}"));
        let expected = fs::read_to_string(fixture_dir.join("expected-substrings.txt"))
            .unwrap_or_else(|error| panic!("failed to read fixture expectations: {error}"));

        let html = build_github_html(&input, &fixture_dir, Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| {
                panic!("fixture {} should render: {error}", fixture_dir.display())
            });
        let body = extract_article_body(&html);

        for snippet in expected.lines().map(str::trim).filter(|line| !line.is_empty()) {
            assert!(
                body.contains(snippet),
                "fixture {} missing snippet: {snippet}\nbody:\n{body}",
                fixture_dir.display()
            );
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn gfm_fixtures_render_through_webkit_and_terminal_graphics_path() {
    for fixture_dir in fixture_dirs() {
        let input = fs::read_to_string(fixture_dir.join("input.md"))
            .unwrap_or_else(|error| panic!("failed to read fixture input: {error}"));
        let html = build_github_html(&input, &fixture_dir, Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| {
                panic!("fixture {} should render: {error}", fixture_dir.display())
            });
        let snapshot = render_html_to_png(&html, &fixture_dir, 960).unwrap_or_else(|error| {
            panic!("fixture {} should snapshot: {error}", fixture_dir.display())
        });
        let page = build_graphic_page(&snapshot.png_bytes, 120, 960).unwrap_or_else(|error| {
            panic!("fixture {} should decode graphic page: {error}", fixture_dir.display())
        });
        let slice = viewport_slice(&page, 0, 20, 21.0);

        assert!(
            page.image_width_px > 0,
            "fixture {} width should be non-zero",
            fixture_dir.display()
        );
        assert!(
            page.image_height_px > 0,
            "fixture {} height should be non-zero",
            fixture_dir.display()
        );
        assert!(
            total_rows(&page, 21.0) > 0,
            "fixture {} should occupy at least one terminal row",
            fixture_dir.display()
        );
        assert!(
            slice.source_height_px > 0,
            "fixture {} should produce a visible viewport slice",
            fixture_dir.display()
        );
        assert!(
            slice.rows > 0,
            "fixture {} should place at least one terminal row",
            fixture_dir.display()
        );
    }
}
