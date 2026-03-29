use crate::{
    cli::Theme,
    core::{
        config::MermaidMode,
        document::{BlockKind, CalloutKind, Document},
    },
    io::{
        image_decoder::{self, ImageDecoder, LoadedImage},
        mermaid_cli::MermaidCliRenderer,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedDocument {
    pub lines: Vec<RenderedLine>,
    pub graphics: Vec<RenderedGraphic>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedLine {
    pub plain_text: String,
    pub display_text: String,
    pub kind: RenderedLineKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RenderedLineKind {
    Blank,
    Heading { level: u8 },
    Paragraph,
    List,
    Quote,
    Callout { kind: CalloutKind },
    Code { language: Option<String>, is_fence_delimiter: bool },
    Table,
    Rule,
    Meta,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedGraphic {
    pub line_index: usize,
    pub width_cells: u16,
    pub height_cells: u16,
    pub png_bytes: Vec<u8>,
}

#[must_use]
pub fn render_plain_text(document: &Document, _theme: Theme, mermaid_mode: MermaidMode) -> String {
    let mut lines = Vec::new();
    let image_decoder = ImageDecoder::new();
    let mermaid_renderer = MermaidCliRenderer::from_env();

    for block in &document.blocks {
        match &block.kind {
            BlockKind::Heading { level, text } => {
                lines.push(format!("{} {}", "#".repeat((*level).into()), text));
            }
            BlockKind::Paragraph { text } => lines.extend(text.lines().map(ToOwned::to_owned)),
            BlockKind::List { ordered, items } => {
                for (index, item) in items.iter().enumerate() {
                    lines.push(if *ordered {
                        format!("{}. {item}", index + 1)
                    } else {
                        format!("- {item}")
                    });
                }
            }
            BlockKind::BlockQuote { text } => {
                lines.extend(text.lines().map(|line| format!("> {line}")));
            }
            BlockKind::Callout { kind, body, .. } => {
                lines.push(format!("[{}] {body}", kind.label()));
            }
            BlockKind::CodeFence { language, code } => {
                lines.push(
                    language
                        .as_ref()
                        .map_or_else(|| "```".to_string(), |value| format!("```{value}")),
                );
                lines.extend(code.lines().map(ToOwned::to_owned));
                lines.push("```".to_string());
            }
            BlockKind::Table { rows } => {
                for row in rows {
                    lines.push(format!("| {} |", row.join(" | ")));
                }
            }
            BlockKind::Image { alt, src, .. } => {
                let label = if alt.is_empty() { src } else { alt };
                match image_decoder.load_from_document(&document.path, src) {
                    Ok(image) => lines.push(format!(
                        "[Image: {label} {}x{} -> {}]",
                        image.width,
                        image.height,
                        image.path.display()
                    )),
                    Err(_) => {
                        let image_path = image_decoder::resolve_image_path(&document.path, src);
                        lines.push(format!("[Image missing: {label} -> {}]", image_path.display()));
                    }
                }
            }
            BlockKind::Mermaid { source } => match mermaid_mode {
                MermaidMode::Disabled => lines.push("[Mermaid disabled]".to_string()),
                MermaidMode::Enabled => match mermaid_renderer.render_png(source) {
                    Ok(png_bytes) => match image_decoder.dimensions_from_png_bytes(&png_bytes) {
                        Ok((width, height)) => {
                            lines.push(format!("[Mermaid rendered: {width}x{height}]"));
                        }
                        Err(_) => {
                            lines.push("[Mermaid unavailable: invalid renderer output]".to_string())
                        }
                    },
                    Err(_) => {
                        lines.push("[Mermaid unavailable: renderer not configured]".to_string());
                    }
                },
            },
            BlockKind::Rule => lines.push("---".to_string()),
            BlockKind::Footnote { label, body } => lines.push(format!("[^{label}] {body}")),
        }

        lines.push(String::new());
    }

    lines.join("\n")
}

#[must_use]
pub fn render_document(
    document: &Document,
    theme: Theme,
    mermaid_mode: MermaidMode,
    width_cells: u16,
) -> RenderedDocument {
    let mut lines = Vec::new();
    let mut graphics = Vec::new();
    let image_decoder = ImageDecoder::new();
    let mermaid_renderer = MermaidCliRenderer::from_env();
    let wrap_width = usize::from(width_cells.max(20));

    for block in &document.blocks {
        match &block.kind {
            BlockKind::Heading { level, text } => {
                push_wrapped_display_line(
                    &mut lines,
                    text.clone(),
                    RenderedLineKind::Heading { level: *level },
                    wrap_width,
                );
            }
            BlockKind::Paragraph { text } => {
                push_wrapped_display_line(
                    &mut lines,
                    display_inline_text(text),
                    RenderedLineKind::Paragraph,
                    wrap_width,
                );
            }
            BlockKind::List { ordered, items } => {
                for (index, item) in items.iter().enumerate() {
                    let prefix =
                        if *ordered { format!("{}. ", index + 1) } else { "• ".to_string() };
                    let item_text = display_list_item(item);
                    push_wrapped_prefixed_line(
                        &mut lines,
                        prefix,
                        "  ".to_string(),
                        item_text,
                        RenderedLineKind::List,
                        wrap_width,
                    );
                }
            }
            BlockKind::BlockQuote { text } => {
                push_wrapped_prefixed_line(
                    &mut lines,
                    "│ ".to_string(),
                    "│ ".to_string(),
                    display_inline_text(text),
                    RenderedLineKind::Quote,
                    wrap_width,
                );
            }
            BlockKind::Callout { kind, title, body } => {
                let heading = title.clone().unwrap_or_else(|| kind.label().to_string());
                push_wrapped_prefixed_line(
                    &mut lines,
                    String::new(),
                    "  ".to_string(),
                    heading,
                    RenderedLineKind::Callout { kind: *kind },
                    wrap_width,
                );
                push_wrapped_prefixed_line(
                    &mut lines,
                    String::new(),
                    "  ".to_string(),
                    display_inline_text(body),
                    RenderedLineKind::Callout { kind: *kind },
                    wrap_width,
                );
            }
            BlockKind::CodeFence { language, code } => {
                let language = language.as_deref().map(normalize_language_token);
                lines.push(RenderedLine {
                    plain_text: String::new(),
                    display_text: language.clone().unwrap_or_else(|| "code".to_string()),
                    kind: RenderedLineKind::Code {
                        language: language.clone(),
                        is_fence_delimiter: true,
                    },
                });
                for code_line in code.lines() {
                    lines.push(RenderedLine {
                        plain_text: code_line.to_owned(),
                        display_text: code_line.to_owned(),
                        kind: RenderedLineKind::Code {
                            language: language.clone(),
                            is_fence_delimiter: false,
                        },
                    });
                }
                lines.push(RenderedLine {
                    plain_text: String::new(),
                    display_text: String::new(),
                    kind: RenderedLineKind::Code { language, is_fence_delimiter: true },
                });
            }
            BlockKind::Table { rows } => {
                for line in render_table_display(rows, wrap_width) {
                    lines.push(RenderedLine {
                        plain_text: line.clone(),
                        display_text: line,
                        kind: RenderedLineKind::Table,
                    });
                }
            }
            BlockKind::Image { alt, src, .. } => {
                let label = if alt.is_empty() { src } else { alt };
                match image_decoder.load_from_document(&document.path, src) {
                    Ok(image) => {
                        let display =
                            format!("Image: {} ({}x{})", label, image.width, image.height);
                        push_wrapped_display_line(
                            &mut lines,
                            display,
                            RenderedLineKind::Meta,
                            wrap_width,
                        );
                        push_graphic(&mut lines, &mut graphics, width_cells, &image);
                    }
                    Err(_) => {
                        push_wrapped_display_line(
                            &mut lines,
                            format!("Image unavailable: {label}"),
                            RenderedLineKind::Meta,
                            wrap_width,
                        );
                    }
                }
            }
            BlockKind::Mermaid { source } => match mermaid_mode {
                MermaidMode::Disabled => {
                    push_wrapped_display_line(
                        &mut lines,
                        "Mermaid disabled".to_string(),
                        RenderedLineKind::Meta,
                        wrap_width,
                    );
                }
                MermaidMode::Enabled => match mermaid_renderer.render_png(source) {
                    Ok(png_bytes) => match image_decoder.dimensions_from_png_bytes(&png_bytes) {
                        Ok((width, height)) => {
                            push_wrapped_display_line(
                                &mut lines,
                                format!("Mermaid diagram ({}x{})", width, height),
                                RenderedLineKind::Meta,
                                wrap_width,
                            );
                            push_png_graphic(
                                &mut lines,
                                &mut graphics,
                                width_cells,
                                width,
                                height,
                                png_bytes,
                            );
                        }
                        Err(_) => {
                            push_wrapped_display_line(
                                &mut lines,
                                "Mermaid unavailable: invalid renderer output".to_string(),
                                RenderedLineKind::Meta,
                                wrap_width,
                            );
                        }
                    },
                    Err(_) => {
                        push_wrapped_display_line(
                            &mut lines,
                            "Mermaid unavailable: renderer not configured".to_string(),
                            RenderedLineKind::Meta,
                            wrap_width,
                        );
                    }
                },
            },
            BlockKind::Rule => lines.push(RenderedLine {
                plain_text: "─".repeat(wrap_width),
                display_text: "─".repeat(wrap_width),
                kind: RenderedLineKind::Rule,
            }),
            BlockKind::Footnote { label, body } => {
                push_wrapped_prefixed_line(
                    &mut lines,
                    format!("[{label}] "),
                    " ".repeat(label.chars().count() + 3),
                    display_inline_text(body),
                    RenderedLineKind::Meta,
                    wrap_width,
                );
            }
        }

        lines.push(RenderedLine {
            plain_text: String::new(),
            display_text: String::new(),
            kind: RenderedLineKind::Blank,
        });
    }

    let _ = theme;
    RenderedDocument { lines, graphics }
}

fn push_wrapped_display_line(
    lines: &mut Vec<RenderedLine>,
    text: String,
    kind: RenderedLineKind,
    width: usize,
) {
    for segment in wrap_words(&text, width) {
        lines.push(RenderedLine {
            plain_text: segment.clone(),
            display_text: segment,
            kind: kind.clone(),
        });
    }
}

fn push_wrapped_prefixed_line(
    lines: &mut Vec<RenderedLine>,
    prefix: String,
    continuation_prefix: String,
    text: String,
    kind: RenderedLineKind,
    width: usize,
) {
    let body_width = width
        .saturating_sub(prefix.chars().count().max(continuation_prefix.chars().count()))
        .max(1);
    let segments = wrap_words(&text, body_width);

    for (index, segment) in segments.into_iter().enumerate() {
        let current_prefix = if index == 0 { &prefix } else { &continuation_prefix };
        let display_text = format!("{current_prefix}{segment}");
        lines.push(RenderedLine {
            plain_text: display_text.clone(),
            display_text,
            kind: kind.clone(),
        });
    }
}

fn render_table_display(rows: &[Vec<String>], width: usize) -> Vec<String> {
    if rows.is_empty() {
        return vec!["(empty table)".to_string()];
    }

    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    if columns == 0 {
        return vec!["(empty table)".to_string()];
    }

    let mut widths = vec![3_usize; columns];
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(display_inline_text(cell).chars().count());
        }
    }

    while table_width(&widths) > width {
        let Some((index, cell_width)) =
            widths.iter_mut().enumerate().max_by_key(|(_, cell_width)| **cell_width)
        else {
            break;
        };
        if *cell_width <= 4 {
            break;
        }
        let _ = index;
        *cell_width -= 1;
    }

    let mut lines = Vec::new();
    lines.push(table_border('┌', '┬', '┐', &widths));
    lines.push(table_row(&rows[0], &widths));
    if rows.len() > 1 {
        lines.push(table_border('├', '┼', '┤', &widths));
        for row in rows.iter().skip(1) {
            lines.push(table_row(row, &widths));
        }
    }
    lines.push(table_border('└', '┴', '┘', &widths));
    lines
}

fn table_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + widths.len() * 3 + 1
}

fn table_border(left: char, middle: char, right: char, widths: &[usize]) -> String {
    let mut line = String::new();
    line.push(left);
    for (index, width) in widths.iter().enumerate() {
        line.push_str(&"─".repeat(*width + 2));
        line.push(if index + 1 == widths.len() { right } else { middle });
    }
    line
}

fn table_row(row: &[String], widths: &[usize]) -> String {
    let mut line = String::from("│");
    for (index, width) in widths.iter().enumerate() {
        let cell = row.get(index).cloned().unwrap_or_default();
        let display = truncate_to_width(&display_inline_text(&cell), *width);
        line.push(' ');
        line.push_str(&display);
        line.push_str(&" ".repeat(width.saturating_sub(display.chars().count()) + 1));
        line.push('│');
    }
    line
}

fn display_list_item(item: &str) -> String {
    if let Some(rest) = item.strip_prefix("[x] ") {
        return format!("☑ {}", display_inline_text(rest));
    }
    if let Some(rest) = item.strip_prefix("[ ] ") {
        return format!("☐ {}", display_inline_text(rest));
    }
    display_inline_text(item)
}

fn display_inline_text(text: &str) -> String {
    normalize_spacing(&strip_link_destinations(text))
}

fn strip_link_destinations(text: &str) -> String {
    let mut remaining = text;
    let mut rendered = String::new();

    while let Some(index) = remaining.find(" <") {
        let (before, rest) = remaining.split_at(index);
        let Some(end) = rest.find('>') else {
            rendered.push_str(remaining);
            return rendered;
        };
        let destination = &rest[2..end];
        if destination.starts_with("http://") || destination.starts_with("https://") {
            rendered.push_str(before);
            remaining = &rest[end + 1..];
            continue;
        }

        rendered.push_str(before);
        rendered.push_str(" <");
        remaining = &rest[2..];
    }

    rendered.push_str(remaining);
    rendered
}

fn normalize_spacing(text: &str) -> String {
    let mut normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    for (from, to) in [
        (" .", "."),
        (" ,", ","),
        (" !", "!"),
        (" ?", "?"),
        (" ;", ";"),
        (" :", ":"),
        (" )", ")"),
        (" ]", "]"),
        ("( ", "("),
        ("[ ", "["),
    ] {
        normalized = normalized.replace(from, to);
    }
    normalized
}

fn wrap_words(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let width = width.max(1);
    let mut wrapped = Vec::new();

    for logical_line in text.lines() {
        if logical_line.is_empty() {
            wrapped.push(String::new());
            continue;
        }

        let mut current = String::new();
        for word in logical_line.split_whitespace() {
            let separator = if current.is_empty() { "" } else { " " };
            let next_width = current.chars().count() + separator.len() + word.chars().count();

            if next_width > width && !current.is_empty() {
                wrapped.push(current);
                current = truncate_to_width(word, width);
                continue;
            }

            if word.chars().count() > width && current.is_empty() {
                wrapped.push(truncate_to_width(word, width));
                continue;
            }

            current.push_str(separator);
            current.push_str(word);
        }

        if !current.is_empty() {
            wrapped.push(current);
        }
    }

    if wrapped.is_empty() {
        wrapped.push(String::new());
    }

    wrapped
}

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width.max(1)).collect()
}

fn push_graphic(
    lines: &mut Vec<RenderedLine>,
    graphics: &mut Vec<RenderedGraphic>,
    width_cells: u16,
    image: &LoadedImage,
) {
    push_png_graphic(
        lines,
        graphics,
        width_cells,
        image.width,
        image.height,
        image.png_bytes.clone(),
    );
}

fn push_png_graphic(
    lines: &mut Vec<RenderedLine>,
    graphics: &mut Vec<RenderedGraphic>,
    width_cells: u16,
    width: u32,
    height: u32,
    png_bytes: Vec<u8>,
) {
    let content_width = width_cells.saturating_sub(2).max(1);
    let aspect = height as f32 / width.max(1) as f32;
    let height_cells = ((content_width as f32 * aspect) / 2.0).ceil() as u16;
    let height_cells = height_cells.clamp(1, 18);
    let line_index = lines.len();
    for _ in 0..height_cells {
        lines.push(RenderedLine {
            plain_text: String::new(),
            display_text: String::new(),
            kind: RenderedLineKind::Blank,
        });
    }
    graphics.push(RenderedGraphic {
        line_index,
        width_cells: content_width,
        height_cells,
        png_bytes,
    });
}

fn normalize_language_token(language: &str) -> String {
    language.trim().to_ascii_lowercase()
}
