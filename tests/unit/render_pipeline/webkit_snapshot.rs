#[cfg(target_os = "macos")]
use image::{ImageBuffer, Rgba};
#[cfg(target_os = "macos")]
use mdv::io::webkit_snapshot::{
    SnapshotDiagnostics, SnapshotTypographyDiagnostics, render_html_to_png,
};
#[cfg(target_os = "macos")]
use mdv::{cli::Theme, core::config::MermaidMode, render::github_html::build_github_html};
#[cfg(target_os = "macos")]
use tempfile::tempdir;

#[cfg(target_os = "macos")]
fn gfm_fixture_dir(name: &str) -> std::path::PathBuf {
    std::path::Path::new("tests/fixtures/gfm").join(name)
}

#[cfg(target_os = "macos")]
fn gfm_fixture_input(name: &str) -> String {
    std::fs::read_to_string(gfm_fixture_dir(name).join("input.md"))
        .unwrap_or_else(|error| panic!("fixture {name} should read: {error}"))
}

#[cfg(target_os = "macos")]
fn typography_entry<'a>(
    diagnostics: &'a SnapshotDiagnostics,
    role: &str,
) -> &'a SnapshotTypographyDiagnostics {
    diagnostics
        .typography
        .iter()
        .find(|entry| entry.role == role)
        .unwrap_or_else(|| panic!("missing typography diagnostics for role {role}"))
}

#[cfg(target_os = "macos")]
fn assert_px_close(actual: f64, expected: f64, label: &str) {
    let delta = (actual - expected).abs();
    assert!(delta <= 0.25, "{label} expected {expected}px but got {actual}px");
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_renders_html_to_png() {
    let html = r#"<!DOCTYPE html>
<html>
<body style="margin:0;background:#0d1117;color:#e6edf3;font:16px -apple-system;">
  <main style="padding:24px">
    <h1>Snapshot</h1>
    <p>WebKit should return a PNG.</p>
  </main>
</body>
</html>"#;

    let snapshot = render_html_to_png(html, std::path::Path::new("."), 960)
        .unwrap_or_else(|error| panic!("webkit should render html to png: {error}"));

    assert!(snapshot.png_bytes.starts_with(&[0x89, b'P', b'N', b'G']));
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_preserves_code_block_backgrounds() {
    let html = build_github_html(
        "```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n",
        std::path::Path::new("."),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    let snapshot = render_html_to_png(&html, std::path::Path::new("."), 960)
        .unwrap_or_else(|error| panic!("webkit should render code block snapshot: {error}"));
    let image = image::load_from_memory(&snapshot.png_bytes)
        .unwrap_or_else(|error| panic!("snapshot png should decode: {error}"))
        .to_rgba8();

    let code_block_pixels = image
        .pixels()
        .filter(|pixel| pixel[0] == 0x16 && pixel[1] == 0x1b && pixel[2] == 0x22)
        .count();

    assert!(
        code_block_pixels > 500,
        "code block background should occupy a visible area in the snapshot"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_matches_github_typography_for_headings_and_inline_emphasis() {
    let markdown = r#"# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

Paragraph with **bold**, *italic*, and `inline code`.
"#;
    let cases = [(Theme::Light, "light"), (Theme::Dark, "dark")];

    for (theme, theme_name) in cases {
        let html =
            build_github_html(markdown, std::path::Path::new("."), theme, MermaidMode::Disabled)
                .unwrap_or_else(|error| panic!("html should render for {theme_name}: {error}"));

        let snapshot =
            render_html_to_png(&html, std::path::Path::new("."), 960).unwrap_or_else(|error| {
                panic!("webkit should render typography snapshot for {theme_name}: {error}")
            });

        assert!(snapshot.diagnostics.fonts_ready, "{theme_name} document fonts should be ready");
        assert!(snapshot.diagnostics.prose_font_ready, "{theme_name} prose font should load");
        assert_eq!(snapshot.diagnostics.heading_font_weight, "600");
        assert_eq!(snapshot.diagnostics.strong_font_weight, "600");

        let heading_expectations = [
            ("h1", 32.0, 40.0),
            ("h2", 24.0, 30.0),
            ("h3", 20.0, 25.0),
            ("h4", 16.0, 20.0),
            ("h5", 14.0, 17.5),
            ("h6", 13.6, 17.0),
        ];

        for (role, expected_font_size, expected_line_height) in heading_expectations {
            let entry = typography_entry(&snapshot.diagnostics, role);
            assert!(entry.present, "{theme_name} {role} should be present");
            assert_eq!(entry.font_weight, "600", "{theme_name} {role} weight");
            assert!(
                entry.font_family.contains("Mona Sans VF"),
                "{theme_name} {role} font family should include Mona Sans VF, got {}",
                entry.font_family
            );
            assert_px_close(
                entry.font_size_px,
                expected_font_size,
                &format!("{theme_name} {role} font-size"),
            );
            assert_px_close(
                entry.line_height_px,
                expected_line_height,
                &format!("{theme_name} {role} line-height"),
            );
        }

        let strong = typography_entry(&snapshot.diagnostics, "strong");
        assert!(strong.present, "{theme_name} strong should be present");
        assert_eq!(strong.font_weight, "600", "{theme_name} strong weight");
        assert!(
            strong.font_family.contains("Mona Sans VF"),
            "{theme_name} strong font family should include Mona Sans VF, got {}",
            strong.font_family
        );

        let emphasis = typography_entry(&snapshot.diagnostics, "em");
        assert!(emphasis.present, "{theme_name} em should be present");
        assert_eq!(emphasis.font_style, "italic", "{theme_name} em style");
        assert!(
            emphasis.font_family.contains("Mona Sans VF"),
            "{theme_name} em font family should include Mona Sans VF, got {}",
            emphasis.font_family
        );

        let code = typography_entry(&snapshot.diagnostics, "code");
        assert!(code.present, "{theme_name} code should be present");
        assert!(
            code.font_family.contains("ui-monospace"),
            "{theme_name} code font family should use the GitHub monospace stack, got {}",
            code.font_family
        );
        assert_px_close(code.font_size_px, 13.6, &format!("{theme_name} code font-size"));
        assert_px_close(code.line_height_px, 20.4, &format!("{theme_name} code line-height"));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_renders_local_markdown_images() {
    let dir = tempdir().unwrap_or_else(|error| panic!("temp dir should exist: {error}"));
    let image_path = dir.path().join("pixel.png");
    let pixel = ImageBuffer::from_pixel(4, 4, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    pixel
        .save(&image_path)
        .unwrap_or_else(|error| panic!("fixture image should be written: {error}"));

    let html = build_github_html(
        "## Image\n\n![Fixture](pixel.png)\n",
        dir.path(),
        Theme::Light,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    let snapshot = render_html_to_png(&html, dir.path(), 960)
        .unwrap_or_else(|error| panic!("webkit should render image snapshot: {error}"));
    let image = image::load_from_memory(&snapshot.png_bytes)
        .unwrap_or_else(|error| panic!("snapshot png should decode: {error}"))
        .to_rgba8();

    assert!(
        image.pixels().any(|pixel| pixel[0] > 220 && pixel[1] < 30 && pixel[2] < 30),
        "rendered snapshot should contain pixels from the local markdown image"
    );
    assert!(snapshot.diagnostics.images_ready);
    assert_eq!(snapshot.diagnostics.images.len(), 1);
    assert_eq!(snapshot.diagnostics.images[0].natural_width_px, 4.0);
    assert_eq!(snapshot.diagnostics.images[0].natural_height_px, 4.0);
    assert!(snapshot.diagnostics.images[0].rendered_width_px > 0.0);
    assert!(snapshot.diagnostics.images[0].rendered_height_px > 0.0);
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_renders_html_img_tag_with_local_image() {
    let dir = tempdir().unwrap_or_else(|error| panic!("temp dir should exist: {error}"));
    let sub = dir.path().join("docs");
    std::fs::create_dir_all(&sub)
        .unwrap_or_else(|error| panic!("fixture directory should be created: {error}"));
    let image_path = sub.join("pixel.png");
    let pixel = ImageBuffer::from_pixel(4, 4, Rgba([0_u8, 0_u8, 255_u8, 255_u8]));
    pixel
        .save(&image_path)
        .unwrap_or_else(|error| panic!("fixture image should be written: {error}"));

    let html = build_github_html(
        "<img src=\"docs/pixel.png\" width=\"100\" alt=\"blue pixel\" />\n",
        dir.path(),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    let article_start = html
        .find("<article")
        .unwrap_or_else(|| panic!("article start should exist in rendered html"));
    let article_end = html
        .find("</article>")
        .unwrap_or_else(|| panic!("article end should exist in rendered html"));

    eprintln!("=== HTML img tag test ===");
    eprintln!("{}", &html[article_start..article_end]);
    eprintln!("=== END ===");

    let snapshot = render_html_to_png(&html, dir.path(), 960)
        .unwrap_or_else(|error| panic!("webkit should render html img tag: {error}"));

    assert!(snapshot.diagnostics.images_ready, "images should be ready");
    assert_eq!(snapshot.diagnostics.images.len(), 1, "should have 1 image");
    assert!(
        snapshot.diagnostics.images[0].natural_width_px > 0.0,
        "natural width should be > 0, got {}",
        snapshot.diagnostics.images[0].natural_width_px
    );
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_renders_html_img_from_project_readme() {
    // Reproduce the actual README.md scenario: <img src="docs/demo.gif" ...>
    // with base_dir = project root
    // Simulate what happens when document.path = "README.md" -> parent = ""
    // The fix normalizes "" to "."
    let raw_parent = std::path::Path::new("README.md")
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let project_root = raw_parent;
    let readme_path = project_root.join("README.md");
    if !readme_path.exists() {
        eprintln!("skipping: README.md not found");
        return;
    }
    let demo_path = project_root.join("docs/demo.gif");
    if !demo_path.exists() {
        eprintln!("skipping: docs/demo.gif not found");
        return;
    }

    let source = std::fs::read_to_string(&readme_path)
        .unwrap_or_else(|error| panic!("README should read: {error}"));
    let html = build_github_html(&source, project_root, Theme::Dark, MermaidMode::Disabled)
        .unwrap_or_else(|error| panic!("README html should render: {error}"));

    // Verify the <img> is restored in HTML
    let body_start = html
        .find("<article")
        .unwrap_or_else(|| panic!("article start should exist in README html"));
    let body_end = html
        .find("</article>")
        .unwrap_or_else(|| panic!("article end should exist in README html"));
    let body = &html[body_start..body_end];
    eprintln!(
        "=== README article (first 500 chars) ===\n{}\n=== END ===",
        &body[..body.len().min(500)]
    );
    assert!(
        body.contains(r#"<img src="docs/demo.gif""#),
        "README <img> tag should be restored in github html"
    );

    let snapshot = render_html_to_png(&html, project_root, 960)
        .unwrap_or_else(|error| panic!("README webkit snapshot should render: {error}"));

    // The README hero image should load successfully.
    let demo_assets: Vec<_> =
        snapshot.diagnostics.images.iter().filter(|img| img.source.contains("demo")).collect();
    assert!(
        !demo_assets.is_empty(),
        "should detect README hero image in diagnostics, got: {:?}",
        snapshot.diagnostics.images.iter().map(|i| &i.source).collect::<Vec<_>>()
    );
    assert!(
        demo_assets[0].natural_width_px > 0.0,
        "README hero image natural width should be > 0, got {}. complete={}, source={}",
        demo_assets[0].natural_width_px,
        demo_assets[0].complete,
        demo_assets[0].source,
    );
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_renders_badge_fixture_images_with_nonzero_size() {
    let fixture_dir = gfm_fixture_dir("badges-local");
    let html = build_github_html(
        &gfm_fixture_input("badges-local"),
        &fixture_dir,
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("badge fixture html should render: {error}"));

    let snapshot = render_html_to_png(&html, &fixture_dir, 960)
        .unwrap_or_else(|error| panic!("badge fixture snapshot should render: {error}"));

    assert!(snapshot.diagnostics.images_ready);
    assert_eq!(snapshot.diagnostics.images.len(), 2);

    for asset in &snapshot.diagnostics.images {
        assert!(asset.complete, "{} should complete", asset.source);
        assert!(asset.natural_width_px >= 80.0, "{} should keep badge width", asset.source);
        assert!(asset.natural_height_px >= 20.0, "{} should keep badge height", asset.source);
        assert!(asset.rendered_width_px >= 80.0, "{} should render visibly wide", asset.source);
        assert!(asset.rendered_height_px >= 18.0, "{} should render visibly tall", asset.source);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_renders_centered_block_fixture_assets() {
    let fixture_dir = gfm_fixture_dir("html-centered-blocks");
    let html = build_github_html(
        &gfm_fixture_input("html-centered-blocks"),
        &fixture_dir,
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("centered block fixture html should render: {error}"));

    let snapshot = render_html_to_png(&html, &fixture_dir, 960)
        .unwrap_or_else(|error| panic!("centered block fixture snapshot should render: {error}"));

    assert!(snapshot.diagnostics.images_ready);
    assert_eq!(snapshot.diagnostics.images.len(), 4);

    let centered_pngs: Vec<_> = snapshot
        .diagnostics
        .images
        .iter()
        .filter(|asset| asset.source.contains("pixel.png"))
        .collect();
    assert_eq!(centered_pngs.len(), 2);
    assert!(
        centered_pngs.iter().any(|asset| asset.rendered_width_px >= 200.0),
        "expected at least one wide centered image, got {:?}",
        centered_pngs.iter().map(|asset| asset.rendered_width_px).collect::<Vec<_>>()
    );
    assert!(
        centered_pngs.iter().any(|asset| asset.rendered_width_px >= 300.0),
        "expected the larger centered image to remain visibly wide, got {:?}",
        centered_pngs.iter().map(|asset| asset.rendered_width_px).collect::<Vec<_>>()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_respects_explicit_html_img_height_attributes() {
    let fixture_dir = gfm_fixture_dir("html-centered-blocks");
    let html = build_github_html(
        &gfm_fixture_input("html-centered-blocks"),
        &fixture_dir,
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("centered block fixture html should render: {error}"));

    let snapshot = render_html_to_png(&html, &fixture_dir, 960)
        .unwrap_or_else(|error| panic!("centered block fixture snapshot should render: {error}"));

    let badges: Vec<_> = snapshot
        .diagnostics
        .images
        .iter()
        .filter(|asset| {
            asset.source.contains("badge-ci.svg") || asset.source.contains("badge-license.svg")
        })
        .collect();

    assert_eq!(badges.len(), 2, "expected both badges in diagnostics");

    for badge in badges {
        assert_px_close(
            badge.rendered_height_px,
            28.0,
            &format!("{} rendered height", badge.source),
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_allows_remote_badge_failures_without_aborting_page_render() {
    let fixture_dir = gfm_fixture_dir("badges-remote");
    let html = build_github_html(
        &gfm_fixture_input("badges-remote"),
        &fixture_dir,
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("remote badge fixture html should render: {error}"));

    let snapshot = render_html_to_png(&html, &fixture_dir, 960)
        .unwrap_or_else(|error| panic!("remote badge failure should not abort snapshot: {error}"));
    let image = snapshot
        .diagnostics
        .images
        .first()
        .unwrap_or_else(|| panic!("remote badge diagnostics should be present"));

    assert!(snapshot.png_bytes.starts_with(&[0x89, b'P', b'N', b'G']));
    assert_eq!(image.source, "http://127.0.0.1:1/ci-badge.svg");
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_surfaces_broken_image_diagnostics() {
    let dir = tempdir().unwrap_or_else(|error| panic!("temp dir should exist: {error}"));
    let html = build_github_html(
        "![Missing](missing.png)\n",
        dir.path(),
        Theme::Light,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    let error = match render_html_to_png(&html, dir.path(), 960) {
        Ok(_) => panic!("snapshot should fail"),
        Err(error) => error,
    };
    let message = error.to_string();

    assert!(message.contains("image"), "{message}");
    assert!(message.contains("failed to render"), "{message}");
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_reports_nonzero_mermaid_metrics() {
    let html = r##"<!DOCTYPE html>
<html>
<body style="margin:0;background:#0d1117;color:#e6edf3;">
  <article class="markdown-body">
    <div class="mdv-mermaid" style="display:flex;justify-content:center;padding:16px 0;">
      <svg class="mdv-mermaid-diagram" viewBox="0 0 200 100" style="display:block;max-width:100%;height:auto;">
        <rect width="200" height="100" fill="#1f6feb"></rect>
        <text x="24" y="58" fill="#ffffff" font-size="28">A -&gt; B</text>
      </svg>
    </div>
  </article>
</body>
</html>"##;

    let snapshot = render_html_to_png(html, std::path::Path::new("."), 960)
        .unwrap_or_else(|error| panic!("webkit should render mermaid snapshot: {error}"));

    assert!(snapshot.diagnostics.mermaids_ready);
    assert_eq!(snapshot.diagnostics.mermaids.len(), 1);
    assert_eq!(snapshot.diagnostics.mermaids[0].view_box, "0 0 200 100");
    assert!(snapshot.diagnostics.mermaids[0].rendered_width_px > 0.0);
    assert!(snapshot.diagnostics.mermaids[0].rendered_height_px > 0.0);
    let aspect = snapshot.diagnostics.mermaids[0].rendered_width_px
        / snapshot.diagnostics.mermaids[0].rendered_height_px;
    assert!(aspect > 1.5 && aspect < 2.5, "unexpected mermaid aspect ratio: {aspect}");
}

#[cfg(target_os = "macos")]
#[test]
fn webkit_snapshot_keeps_rich_fixture_image_and_mermaid_visible() {
    let source = std::fs::read_to_string("examples/rich_markdown.md")
        .unwrap_or_else(|error| panic!("fixture should read: {error}"));
    let html = build_github_html(
        &source,
        std::path::Path::new("examples"),
        Theme::Dark,
        MermaidMode::Enabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    if html.contains("Mermaid unavailable") {
        return;
    }

    let snapshot = render_html_to_png(&html, std::path::Path::new("examples"), 960)
        .unwrap_or_else(|error| panic!("webkit should render rich fixture snapshot: {error}"));

    let image = snapshot
        .diagnostics
        .images
        .iter()
        .find(|asset| asset.source.contains("pixel.png"))
        .unwrap_or_else(|| panic!("rich fixture image diagnostics missing"));
    assert!(image.natural_width_px > 0.0);
    assert!(
        image.rendered_width_px >= 32.0,
        "rich fixture image should stay visibly large in the rendered snapshot, got {}px",
        image.rendered_width_px
    );
    assert!(
        image.rendered_height_px >= 32.0,
        "rich fixture image should stay visibly tall in the rendered snapshot, got {}px",
        image.rendered_height_px
    );

    let mermaid = snapshot
        .diagnostics
        .mermaids
        .first()
        .unwrap_or_else(|| panic!("rich fixture mermaid diagnostics missing"));
    assert!(mermaid.rendered_width_px > 0.0);
    assert!(mermaid.rendered_height_px > 0.0);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn webkit_snapshot_is_mac_only() {}
