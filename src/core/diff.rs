use crate::core::document::Document;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlockDiff {
    pub inserted: Vec<usize>,
    pub removed: Vec<usize>,
    pub updated: Vec<usize>,
    pub first_dirty_index: Option<usize>,
}

impl BlockDiff {
    #[must_use]
    pub fn between(old: &Document, new: &Document) -> Self {
        let mut diff = Self::default();
        let max_len = old.blocks.len().max(new.blocks.len());

        for index in 0..max_len {
            match (old.blocks.get(index), new.blocks.get(index)) {
                (Some(previous), Some(current)) if previous.source_hash != current.source_hash => {
                    diff.updated.push(index);
                    diff.first_dirty_index.get_or_insert(index);
                }
                (None, Some(_)) => {
                    diff.inserted.push(index);
                    diff.first_dirty_index.get_or_insert(index);
                }
                (Some(_), None) => {
                    diff.removed.push(index);
                    diff.first_dirty_index.get_or_insert(index);
                }
                _ => {}
            }
        }

        diff
    }
}
