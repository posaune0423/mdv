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
    const CHILD: &str = "MDV_TEST_PLAIN_TEXT_MERMAID_UNAVAILABLE";
    if std::env::var_os(CHILD).is_none() {
        let directory = tempfile::tempdir()
            .unwrap_or_else(|error| panic!("missing renderer fixture should create: {error}"));
        let test_executable = std::env::current_exe()
            .unwrap_or_else(|error| panic!("test executable should resolve: {error}"));
        assert_cmd::Command::new(test_executable)
            .args([
                "--exact",
                "unit::plain_text_render::renders_image_state_and_mermaid_unavailable_reason",
            ])
            .env(CHILD, "1")
            .env("MDV_MERMAID_CMD", directory.path().join("missing-mermaid-executable"))
            .timeout(std::time::Duration::from_secs(15))
            .assert()
            .success();
        return;
    }

    let document = match parse_document(
        "docs/example.md".into(),
        "![diagram](docs/missing.png)\n\n```mermaid\ngraph TD\n    A --> B\n```\n",
    ) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    let rendered = render_plain_text(&document, Theme::Light, MermaidMode::Enabled);

    assert!(rendered.contains("[Image missing: diagram"));
    assert!(rendered.contains("[Mermaid unavailable: renderer not configured]"));
}
