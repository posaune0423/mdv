use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use anyhow::Result;
use comrak::{
    Arena,
    nodes::{AlertType, AstNode, NodeCodeBlock, NodeLink, NodeList, NodeValue},
    parse_document as parse_comrak_document,
};

use super::markdown_pipeline::gfm_options;
use crate::core::document::{
    Block, BlockId, BlockKind, CalloutKind, Document, DocumentMeta, InlineSegment, InlineStyle,
    SourceSpan, StyledText,
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
        BlockKind::Heading { text, .. } => Some(text.plain()),
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
            Some(BlockKind::Heading { level: heading.level, text: collect_styled_text(node) })
        }
        NodeValue::Paragraph => normalize_paragraph(node),
        NodeValue::BlockQuote => Some(normalize_quote(node)),
        NodeValue::List(list) => Some(normalize_list(node, list)),
        NodeValue::CodeBlock(code) => Some(normalize_code_block(code)),
        NodeValue::HtmlBlock(block) => normalize_html_block(&block.literal),
        NodeValue::ThematicBreak => Some(BlockKind::Rule),
        NodeValue::Table(_) => Some(normalize_table(node)),
        NodeValue::FootnoteDefinition(definition) => Some(BlockKind::Footnote {
            label: definition.name.clone(),
            body: collect_styled_text(node),
        }),
        NodeValue::Alert(alert) => Some(BlockKind::Callout {
            kind: map_alert_kind(alert.alert_type),
            title: alert.title.clone().map(StyledText::from_plain),
            body: collect_styled_text(node),
        }),
        _ => None,
    }
}

fn normalize_paragraph<'a>(node: &'a AstNode<'a>) -> Option<BlockKind> {
    if let Some(image) = extract_image(node) {
        return Some(image);
    }

    let text = collect_styled_text(node);
    (!text.is_empty()).then_some(BlockKind::Paragraph { text })
}

fn normalize_quote<'a>(node: &'a AstNode<'a>) -> BlockKind {
    let text = collect_styled_text(node);
    if let Some((kind, body)) = parse_callout(&text.plain()) {
        return BlockKind::Callout { kind, title: None, body };
    }

    BlockKind::BlockQuote { text }
}

fn normalize_list<'a>(node: &'a AstNode<'a>, list: &NodeList) -> BlockKind {
    let items = node.children().map(collect_styled_text).filter(|text| !text.is_empty()).collect();

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
            row.children()
                .map(collect_styled_text)
                .filter(|cell| !cell.is_empty())
                .collect::<Vec<_>>()
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
        alt: collect_styled_text(image).plain(),
        title: (!link.title.is_empty()).then(|| link.title.clone()),
    }
}

fn parse_callout(text: &str) -> Option<(CalloutKind, StyledText)> {
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

    Some((kind, StyledText::from_plain(body.trim().to_string())))
}

fn collect_styled_text<'a>(node: &'a AstNode<'a>) -> StyledText {
    let mut segments = Vec::new();
    collect_text_segments(node, InlineStyle::default(), &mut segments);
    StyledText { segments: normalize_segments(segments) }
}

fn collect_text_segments<'a>(
    node: &'a AstNode<'a>,
    style: InlineStyle,
    parts: &mut Vec<InlineSegment>,
) {
    match &node.data.borrow().value {
        NodeValue::Text(text) => {
            let value = text.to_string();
            if !value.is_empty() {
                parts.push(InlineSegment { text: value, style });
            }
        }
        NodeValue::Code(code) => {
            let value = code.literal.clone();
            if !value.is_empty() {
                parts.push(InlineSegment {
                    text: value,
                    style: InlineStyle { code: true, ..style },
                });
            }
        }
        NodeValue::HtmlInline(literal) => {
            if !literal.is_empty() {
                parts.push(InlineSegment { text: literal.clone(), style });
            }
        }
        NodeValue::TaskItem(task) => {
            let marker = task.symbol.map_or("[ ] ".to_string(), |value| format!("[{value}] "));
            parts.push(InlineSegment { text: marker, style });
            for child in node.children() {
                collect_text_segments(child, style, parts);
            }
        }
        NodeValue::FootnoteReference(reference) => {
            parts.push(InlineSegment { text: format!("[^{}]", reference.name), style });
        }
        NodeValue::Link(link) => {
            for child in node.children() {
                collect_text_segments(child, style, parts);
            }
            if !link.url.is_empty() {
                parts.push(InlineSegment { text: format!(" <{}>", link.url), style });
            }
        }
        NodeValue::Strong => {
            let style = InlineStyle { bold: true, ..style };
            for child in node.children() {
                collect_text_segments(child, style, parts);
            }
        }
        NodeValue::Emph => {
            let style = InlineStyle { italic: true, ..style };
            for child in node.children() {
                collect_text_segments(child, style, parts);
            }
        }
        NodeValue::LineBreak | NodeValue::SoftBreak => {
            parts.push(InlineSegment { text: "\n".to_string(), style });
        }
        NodeValue::Image(_) => {
            for child in node.children() {
                collect_text_segments(child, style, parts);
            }
        }
        _ => {
            for child in node.children() {
                collect_text_segments(child, style, parts);
            }
        }
    }
}

fn hash_block(kind: &BlockKind) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    hasher.finish()
}

fn normalize_segments(parts: Vec<InlineSegment>) -> Vec<InlineSegment> {
    let mut normalized: Vec<InlineSegment> = Vec::new();

    for part in parts {
        if part.text.is_empty() {
            continue;
        }
        if let Some(previous) = normalized.last_mut()
            && previous.style == part.style
        {
            previous.text.push_str(&part.text);
            continue;
        }
        normalized.push(part);
    }

    normalized
}

fn normalize_html_block(literal: &str) -> Option<BlockKind> {
    if let Some(image) = extract_html_img(literal) {
        return Some(image);
    }

    let trimmed = literal.trim();
    if is_layout_only_html(trimmed) {
        return None;
    }

    let text = StyledText::from_plain(literal.trim_end_matches(['\r', '\n']).to_string());
    (!text.is_empty()).then_some(BlockKind::Paragraph { text })
}

fn is_layout_only_html(html: &str) -> bool {
    let tag = html
        .trim_start_matches('<')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("");
    matches!(tag, "div" | "p" | "br" | "center" | "details" | "summary" | "section" | "span")
}

fn extract_html_img(html: &str) -> Option<BlockKind> {
    let trimmed = html.trim();
    let img_start = trimmed.find("<img")?;
    let after_img = trimmed[img_start + 4..].trim();
    let img_end = after_img.find("/>").or_else(|| after_img.find('>'))?;
    let img_content = after_img[..img_end].trim();

    let src = extract_html_attr(img_content, "src")?;
    let alt = extract_html_attr(img_content, "alt").unwrap_or_default();
    let title = extract_html_attr(img_content, "title");

    Some(BlockKind::Image { src, alt, title })
}

fn extract_html_attr(html: &str, name: &str) -> Option<String> {
    let needle = format!(r#"{name}=""#);
    let start = html.find(&needle)? + needle.len();
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
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
