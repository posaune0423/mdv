use mdv::{
    cli::Theme,
    core::config::MermaidMode,
    render::{
        markdown::parse_document,
        text::{RenderedGraphicContent, RenderedLineKind, render_document},
    },
};

#[test]
fn terminal_render_strips_raw_markdown_markers_from_headings_and_quotes() {
    let document = parse_document(
        "docs/example.md".into(),
        "# Title\n\n> quoted text\n\n```rust\nfn main() {}\n```\n",
    )
    .unwrap_or_else(|error| panic!("document should parse: {error}"));

    let rendered = render_document(&document, Theme::Light, MermaidMode::Enabled, 40);

    assert_eq!(rendered.lines[0].display_text, "Title");
    assert_eq!(rendered.lines[0].kind, RenderedLineKind::Heading { level: 1 });
    assert_eq!(rendered.lines[2].display_text, "│ quoted text");
    assert_eq!(rendered.lines[2].kind, RenderedLineKind::Quote);
    assert_eq!(rendered.lines[4].display_text, "rust");
    assert_eq!(
        rendered.lines[4].kind,
        RenderedLineKind::Code { language: Some("rust".to_string()), is_fence_delimiter: true }
    );
}

#[test]
fn terminal_render_wraps_long_lines_to_view_width() {
    let document = parse_document(
        "docs/example.md".into(),
        "This paragraph is intentionally long so it must wrap inside the terminal viewer.\n",
    )
    .unwrap_or_else(|error| panic!("document should parse: {error}"));

    let rendered = render_document(&document, Theme::Light, MermaidMode::Enabled, 20);

    assert!(rendered.lines.len() > 2);
    assert!(rendered.lines[0].display_text.chars().count() <= 20);
    assert!(rendered.lines[1].display_text.chars().count() <= 20);
}

#[test]
fn terminal_render_carries_code_language_into_code_lines() {
    let document = parse_document("docs/example.md".into(), "```rust\nlet answer = 42;\n```\n")
        .unwrap_or_else(|error| panic!("document should parse: {error}"));

    let rendered = render_document(&document, Theme::Light, MermaidMode::Enabled, 40);

    assert_eq!(
        rendered.lines[1].kind,
        RenderedLineKind::Code { language: Some("rust".to_string()), is_fence_delimiter: false }
    );
}

#[test]
fn terminal_render_formats_lists_links_and_tables_for_interactive_view() {
    let document = parse_document(
        "docs/example.md".into(),
        "Paragraph with a [link](https://example.com).\n\n- bullet item\n- [x] done\n- [ ] pending\n\n| Name | Value |\n| --- | --- |\n| alpha | beta |\n",
    )
    .unwrap_or_else(|error| panic!("document should parse: {error}"));

    let rendered = render_document(&document, Theme::Light, MermaidMode::Enabled, 48);
    let display_lines =
        rendered.lines.iter().map(|line| line.display_text.clone()).collect::<Vec<_>>();

    assert!(display_lines.iter().any(|line| line.contains("Paragraph with a link.")));
    assert!(display_lines.iter().all(|line| !line.contains("<https://example.com>")));
    assert!(display_lines.iter().any(|line| line.contains("• bullet item")));
    assert!(display_lines.iter().any(|line| line.contains("☑ done")));
    assert!(display_lines.iter().any(|line| line.contains("☐ pending")));
    assert!(display_lines.iter().any(|line| line.contains("┌")));
    assert!(display_lines.iter().any(|line| line.contains("Name")));
    assert!(display_lines.iter().any(|line| line.contains("alpha")));
}

#[test]
fn terminal_render_defers_mermaid_rasterization_for_interactive_view() {
    let document = parse_document(
        "docs/example.md".into(),
        "```mermaid\ngraph TD\n    A --> B\n```\n",
    )
    .unwrap_or_else(|error| panic!("document should parse: {error}"));

    let rendered = render_document(&document, Theme::Light, MermaidMode::Enabled, 48);
    let graphic = rendered.graphics.first().expect("mermaid graphic should exist");

    assert!(matches!(
        &graphic.content,
        RenderedGraphicContent::Mermaid { png_bytes: None, .. }
    ));
}
