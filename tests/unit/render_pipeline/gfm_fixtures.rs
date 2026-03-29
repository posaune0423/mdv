use std::{fs, path::Path};

use mdv::{
    cli::Theme,
    core::config::MermaidMode,
    render::github_html::build_github_html,
    ui::page_graphics::{build_graphic_page, total_rows, viewport_slice},
};

#[cfg(target_os = "macos")]
use mdv::io::webkit_snapshot::render_html_to_png;

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
