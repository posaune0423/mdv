use mdv::{
    cli::Theme,
    core::config::MermaidMode,
    render::{markdown::parse_document, text::render_plain_text},
};

#[test]
fn renders_links_inline_with_destination() {
    let document = match parse_document(
        "docs/example.md".into(),
        "Paragraph with a [link](https://example.com).\n",
    ) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    let rendered = render_plain_text(&document, Theme::Light, MermaidMode::Enabled);

    assert!(rendered.contains("link <https://example.com>"));
}

#[test]
fn renders_image_state_and_mermaid_unavailable_reason() {
    let document = match parse_document(
        "docs/example.md".into(),
        "![diagram](docs/missing.png)\n\n```mermaid\ngraph TD\n    A --> B\n```\n",
    ) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    let rendered = render_plain_text(&document, Theme::Light, MermaidMode::Enabled);

    assert!(rendered.contains("[Image missing: diagram"));
    assert!(rendered.contains("[Mermaid"));
}
