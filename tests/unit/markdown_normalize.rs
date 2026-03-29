use mdv::core::document::BlockKind;
use mdv::render::markdown::parse_document;

#[test]
fn normalizes_common_blocks_from_gfm_markdown() {
    let source = r#"# Heading

Paragraph with a [link](https://example.com).

> [!NOTE]
> Callout body

```mermaid
graph TD
    A --> B
```
"#;

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(document.blocks[0].kind, BlockKind::Heading { level: 1, .. }));
    assert!(matches!(document.blocks[1].kind, BlockKind::Paragraph { .. }));
    assert!(matches!(document.blocks[2].kind, BlockKind::Callout { .. }));
    assert!(matches!(document.blocks[3].kind, BlockKind::Mermaid { .. }));
    assert_eq!(document.meta.links, vec!["https://example.com".to_string()]);
}

#[test]
fn normalizes_task_list_and_footnote_reference_text() {
    let source = "- [x] done\n- [ ] pending\n\nFootnote ref[^1].\n\n[^1]: detail\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(
        document.blocks[0].kind,
        BlockKind::List { ref items, .. }
            if items
                == &vec![
                    mdv::core::document::StyledText::from_plain("[x] done"),
                    mdv::core::document::StyledText::from_plain("[ ] pending"),
                ]
    ));
    assert!(matches!(
        document.blocks[1].kind,
        BlockKind::Paragraph { ref text } if text.plain().contains("[^1]")
    ));
    assert!(matches!(document.blocks[2].kind, BlockKind::Footnote { .. }));
}

#[test]
fn skips_layout_only_html_blocks() {
    let source = "<div align=\"center\">\n</div>\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(
        document.blocks.is_empty(),
        "layout-only HTML blocks (<div>, </div>) should be skipped, got: {:?}",
        document.blocks.iter().map(|b| &b.kind).collect::<Vec<_>>()
    );
}

#[test]
fn preserves_non_layout_html_blocks_as_plain_text() {
    let source = "<table><tr><td>cell</td></tr></table>\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(
        document.blocks.as_slice(),
        [mdv::core::document::Block {
            kind: BlockKind::Paragraph { .. },
            ..
        }]
    ));
}

#[test]
fn parses_html_img_tag_as_image_block() {
    let source = "<img src=\"photo.png\" alt=\"A photo\" />\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(
        document.blocks.as_slice(),
        [mdv::core::document::Block {
            kind: BlockKind::Image { src, alt, .. },
            ..
        }] if src == "photo.png" && alt == "A photo"
    ));
}

#[test]
fn parses_html_img_wrapped_in_p_as_image_block() {
    let source = "<p align=\"center\">\n  <img src=\"hero.jpg\" alt=\"Centered\" />\n</p>\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(
        document.blocks.as_slice(),
        [mdv::core::document::Block {
            kind: BlockKind::Image { src, alt, .. },
            ..
        }] if src == "hero.jpg" && alt == "Centered"
    ));
}

#[test]
fn parses_html_img_wrapped_in_div_as_image_block() {
    let source = "<div align=\"center\">\n<img src=\"banner.png\" alt=\"Banner\" />\n</div>\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(
        document.blocks.as_slice(),
        [mdv::core::document::Block {
            kind: BlockKind::Image { src, alt, .. },
            ..
        }] if src == "banner.png" && alt == "Banner"
    ));
}

#[test]
fn parses_html_img_tag_with_width_as_image_block() {
    let source = "<img src=\"hero.jpg\" width=\"700\" alt=\"Hero\" />\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(
        document.blocks.as_slice(),
        [mdv::core::document::Block {
            kind: BlockKind::Image { src, alt, .. },
            ..
        }] if src == "hero.jpg" && alt == "Hero"
    ));
}

#[test]
fn keeps_inline_html_as_plain_text() {
    let source = "Before <br/> after.\n";

    let document = match parse_document("docs/example.md".into(), source) {
        Ok(document) => document,
        Err(error) => panic!("document should parse: {error}"),
    };

    assert!(matches!(
        document.blocks.as_slice(),
        [mdv::core::document::Block {
            kind: BlockKind::Paragraph { text },
            ..
        }] if text.plain() == "Before <br/> after."
    ));
}
