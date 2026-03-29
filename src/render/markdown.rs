use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use anyhow::Result;
use comrak::{
    Arena, Options,
    nodes::{AlertType, AstNode, NodeCodeBlock, NodeLink, NodeList, NodeValue},
    parse_document as parse_comrak_document,
};

use crate::core::document::{
    Block, BlockId, BlockKind, CalloutKind, Document, DocumentMeta, SourceSpan,
};

pub fn parse_document(path: PathBuf, source: &str) -> Result<Document> {
    let arena = Arena::new();
    let root = parse_comrak_document(&arena, source, &gfm_options());
    let mut blocks = Vec::new();

    for (index, node) in root.children().enumerate() {
        if let Some(kind) = normalize_block(node) {
            let source_hash = hash_block(&kind);
            blocks.push(Block {
                id: BlockId(index),
                kind,
                span: SourceSpan::default(),
                source_hash,
            });
        }
    }

    let title = blocks.iter().find_map(|block| match &block.kind {
        BlockKind::Heading { text, .. } => Some(text.clone()),
        _ => None,
    });
    let mut links = Vec::new();
    collect_links(root, &mut links);

    Ok(Document {
        path,
        revision: 0,
        blocks,
        meta: DocumentMeta { title, links, source_len: source.len() },
    })
}

fn normalize_block<'a>(node: &'a AstNode<'a>) -> Option<BlockKind> {
    match &node.data.borrow().value {
        NodeValue::Heading(heading) => {
            Some(BlockKind::Heading { level: heading.level, text: collect_text(node) })
        }
        NodeValue::Paragraph => normalize_paragraph(node),
        NodeValue::BlockQuote => Some(normalize_quote(node)),
        NodeValue::List(list) => Some(normalize_list(node, list)),
        NodeValue::CodeBlock(code) => Some(normalize_code_block(code)),
        NodeValue::HtmlBlock(_) => {
            let text = collect_text(node);
            (!text.is_empty()).then_some(BlockKind::Paragraph { text })
        }
        NodeValue::ThematicBreak => Some(BlockKind::Rule),
        NodeValue::Table(_) => Some(normalize_table(node)),
        NodeValue::FootnoteDefinition(definition) => {
            Some(BlockKind::Footnote { label: definition.name.clone(), body: collect_text(node) })
        }
        NodeValue::Alert(alert) => Some(BlockKind::Callout {
            kind: map_alert_kind(alert.alert_type),
            title: alert.title.clone(),
            body: collect_text(node),
        }),
        _ => None,
    }
}

fn normalize_paragraph<'a>(node: &'a AstNode<'a>) -> Option<BlockKind> {
    if let Some(image) = extract_image(node) {
        return Some(image);
    }

    let text = collect_text(node);
    (!text.is_empty()).then_some(BlockKind::Paragraph { text })
}

fn normalize_quote<'a>(node: &'a AstNode<'a>) -> BlockKind {
    let text = collect_text(node);
    if let Some((kind, body)) = parse_callout(&text) {
        return BlockKind::Callout { kind, title: None, body };
    }

    BlockKind::BlockQuote { text }
}

fn normalize_list<'a>(node: &'a AstNode<'a>, list: &NodeList) -> BlockKind {
    let items = node.children().map(collect_text).filter(|text| !text.is_empty()).collect();

    BlockKind::List { ordered: list.list_type == comrak::nodes::ListType::Ordered, items }
}

fn normalize_code_block(code: &NodeCodeBlock) -> BlockKind {
    let language = code.info.trim().to_string();
    let literal = code.literal.clone();

    if language == "mermaid" {
        BlockKind::Mermaid { source: literal }
    } else {
        BlockKind::CodeFence { language: (!language.is_empty()).then_some(language), code: literal }
    }
}

fn normalize_table<'a>(node: &'a AstNode<'a>) -> BlockKind {
    let rows = node
        .children()
        .map(|row| {
            row.children().map(collect_text).filter(|cell| !cell.is_empty()).collect::<Vec<_>>()
        })
        .filter(|row| !row.is_empty())
        .collect();

    BlockKind::Table { rows }
}

fn extract_image<'a>(node: &'a AstNode<'a>) -> Option<BlockKind> {
    let mut children = node.children();
    let image = children.next()?;
    if children.next().is_some() {
        return None;
    }

    match &image.data.borrow().value {
        NodeValue::Image(link) => Some(image_block(link, image)),
        _ => None,
    }
}

fn image_block<'a>(link: &NodeLink, image: &'a AstNode<'a>) -> BlockKind {
    BlockKind::Image {
        src: link.url.clone(),
        alt: collect_text(image),
        title: (!link.title.is_empty()).then(|| link.title.clone()),
    }
}

fn parse_callout(text: &str) -> Option<(CalloutKind, String)> {
    let trimmed = text.trim();
    let (tag, body) = trimmed.split_once(']')?;
    let kind = match tag {
        "[!NOTE" => CalloutKind::Note,
        "[!TIP" => CalloutKind::Tip,
        "[!IMPORTANT" => CalloutKind::Important,
        "[!WARNING" => CalloutKind::Warning,
        "[!CAUTION" => CalloutKind::Caution,
        _ => return None,
    };

    Some((kind, body.trim().to_string()))
}

fn collect_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut parts = Vec::new();
    collect_text_parts(node, &mut parts);
    normalize_text_parts(parts)
}

fn collect_text_parts<'a>(node: &'a AstNode<'a>, parts: &mut Vec<String>) {
    match &node.data.borrow().value {
        NodeValue::Text(text) => {
            let value = text.trim().to_string();
            if !value.is_empty() {
                parts.push(value);
            }
        }
        NodeValue::Code(code) => {
            let value = code.literal.trim().to_string();
            if !value.is_empty() {
                parts.push(value);
            }
        }
        NodeValue::TaskItem(task) => {
            let marker = task.symbol.map_or("[ ]".to_string(), |value| format!("[{value}]"));
            parts.push(marker);
            for child in node.children() {
                collect_text_parts(child, parts);
            }
        }
        NodeValue::FootnoteReference(reference) => {
            parts.push(format!("[^{}]", reference.name));
        }
        NodeValue::Link(link) => {
            let mut label_parts = Vec::new();
            for child in node.children() {
                collect_text_parts(child, &mut label_parts);
            }
            let label = normalize_text_parts(label_parts);
            if label.is_empty() {
                parts.push(format!("<{}>", link.url));
            } else {
                parts.push(format!("{label} <{}>", link.url));
            }
        }
        NodeValue::LineBreak | NodeValue::SoftBreak => parts.push("\n".to_string()),
        NodeValue::Image(_) => {
            for child in node.children() {
                collect_text_parts(child, parts);
            }
        }
        _ => {
            for child in node.children() {
                collect_text_parts(child, parts);
            }
        }
    }
}

fn hash_block(kind: &BlockKind) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    hasher.finish()
}

fn normalize_text_parts(parts: Vec<String>) -> String {
    parts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_links<'a>(node: &'a AstNode<'a>, links: &mut Vec<String>) {
    if let NodeValue::Link(link) = &node.data.borrow().value
        && !links.contains(&link.url)
    {
        links.push(link.url.clone());
    }

    for child in node.children() {
        collect_links(child, links);
    }
}

fn map_alert_kind(kind: AlertType) -> CalloutKind {
    match kind {
        AlertType::Note => CalloutKind::Note,
        AlertType::Tip => CalloutKind::Tip,
        AlertType::Important => CalloutKind::Important,
        AlertType::Warning => CalloutKind::Warning,
        AlertType::Caution => CalloutKind::Caution,
    }
}

fn gfm_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.alerts = true;
    options
}
