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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct InlineSegment {
    pub text: String,
    pub style: InlineStyle,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct StyledText {
    pub segments: Vec<InlineSegment>,
}

impl StyledText {
    #[must_use]
    pub fn plain(&self) -> String {
        self.segments.iter().map(|segment| segment.text.as_str()).collect()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|segment| segment.text.is_empty())
    }

    #[must_use]
    pub fn from_plain(text: impl Into<String>) -> Self {
        let text = text.into();
        if text.is_empty() {
            return Self::default();
        }
        Self { segments: vec![InlineSegment { text, style: InlineStyle::default() }] }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum BlockKind {
    Heading { level: u8, text: StyledText },
    Paragraph { text: StyledText },
    List { ordered: bool, items: Vec<StyledText> },
    BlockQuote { text: StyledText },
    Callout { kind: CalloutKind, title: Option<StyledText>, body: StyledText },
    CodeFence { language: Option<String>, code: String },
    Table { rows: Vec<Vec<StyledText>> },
    Image { src: String, alt: String, title: Option<String> },
    Mermaid { source: String },
    Rule,
    Footnote { label: String, body: StyledText },
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
