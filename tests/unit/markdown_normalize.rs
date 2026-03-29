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
