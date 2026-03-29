use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Document {
    pub path: PathBuf,
    pub revision: u64,
    pub blocks: Vec<Block>,
    pub meta: DocumentMeta,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocumentMeta {
    pub title: Option<String>,
    pub links: Vec<String>,
    pub source_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub kind: BlockKind,
    pub span: SourceSpan,
    pub source_hash: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlockId(pub usize);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlockKind {
    Heading { level: u8, text: String },
    Paragraph { text: String },
    List { ordered: bool, items: Vec<String> },
    BlockQuote { text: String },
    Callout { kind: CalloutKind, title: Option<String>, body: String },
    CodeFence { language: Option<String>, code: String },
    Table { rows: Vec<Vec<String>> },
    Image { src: String, alt: String, title: Option<String> },
    Mermaid { source: String },
    Rule,
    Footnote { label: String, body: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CalloutKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl CalloutKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Note => "Note",
            Self::Tip => "Tip",
            Self::Important => "Important",
            Self::Warning => "Warning",
            Self::Caution => "Caution",
        }
    }
}
