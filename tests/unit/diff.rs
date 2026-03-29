use mdv::core::{
    diff::BlockDiff,
    document::{Block, BlockId, BlockKind, Document, DocumentMeta, SourceSpan},
};

#[test]
fn reports_first_dirty_index_when_document_changes_midway() {
    let old = sample_document(vec!["first", "second", "third"]);
    let new = sample_document(vec!["first", "changed", "third"]);

    let diff = BlockDiff::between(&old, &new);

    assert_eq!(diff.first_dirty_index, Some(1));
    assert_eq!(diff.updated, vec![1]);
}

fn sample_document(lines: Vec<&str>) -> Document {
    Document {
        path: "docs/example.md".into(),
        revision: 0,
        blocks: lines
            .into_iter()
            .enumerate()
            .map(|(index, value)| Block {
                id: BlockId(index),
                kind: BlockKind::Paragraph { text: value.to_string() },
                span: SourceSpan::default(),
                source_hash: index as u64 ^ (value.len() as u64),
            })
            .collect(),
        meta: DocumentMeta::default(),
    }
}
