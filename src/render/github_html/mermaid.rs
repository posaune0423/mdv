use std::{collections::HashMap, thread};

use anyhow::Result;
use comrak::nodes::{AstNode, NodeValue};

use crate::{core::config::MermaidMode, io::mermaid_cli::MermaidCliRenderer};

pub(super) fn replace_mermaid_code_blocks<'a>(
    root: &'a AstNode<'a>,
    mermaid_mode: MermaidMode,
    mermaid_renderer: &MermaidCliRenderer,
) -> Result<()> {
    let mermaid_sources = root
        .descendants()
        .filter_map(|node| {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::CodeBlock(code) if code.info.trim().eq_ignore_ascii_case("mermaid") => {
                    Some(code.literal.clone())
                }
                _ => None,
            }
        })
        .fold(Vec::<String>::new(), |mut acc, source| {
            if !acc.contains(&source) {
                acc.push(source);
            }
            acc
        });
    let rendered_mermaid = render_mermaid_blocks_with(&mermaid_sources, mermaid_mode, |source| {
        mermaid_renderer.render_svg_sized(source, Some(690), Some(2.0))
    })?;

    for node in root.descendants() {
        let mermaid_source = {
            let data = node.data.borrow();
            match &data.value {
                NodeValue::CodeBlock(code) if code.info.trim().eq_ignore_ascii_case("mermaid") => {
                    Some(code.literal.clone())
                }
                _ => None,
            }
        };

        if let Some(source) = mermaid_source {
            let rendered = rendered_mermaid.get(&source).cloned().unwrap_or_else(|| {
                "<div class=\"mdv-mermaid mdv-mermaid-fallback\">Mermaid unavailable</div>"
                    .to_string()
            });
            node.data.borrow_mut().value = NodeValue::Raw(rendered);
        }
    }

    Ok(())
}

pub(super) fn render_mermaid_blocks_with<F>(
    sources: &[String],
    mermaid_mode: MermaidMode,
    render_mermaid: F,
) -> Result<HashMap<String, String>>
where
    F: Fn(&str) -> Result<String> + Sync,
{
    let unique_sources = sources.iter().fold(Vec::<String>::new(), |mut acc, source| {
        if !acc.contains(source) {
            acc.push(source.clone());
        }
        acc
    });

    if unique_sources.is_empty() {
        return Ok(HashMap::new());
    }

    let worker_count = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(4)
        .min(unique_sources.len())
        .max(1);
    let chunk_size = unique_sources.len().div_ceil(worker_count);
    let mut rendered = HashMap::new();

    thread::scope(|scope| -> Result<()> {
        let mut handles = Vec::new();
        for chunk in unique_sources.chunks(chunk_size) {
            let render_mermaid = &render_mermaid;
            handles.push(scope.spawn(move || -> Result<Vec<(String, String)>> {
                let mut chunk_results = Vec::with_capacity(chunk.len());
                for source in chunk {
                    chunk_results.push((
                        source.clone(),
                        render_mermaid_block_with(source, mermaid_mode, render_mermaid)?,
                    ));
                }
                Ok(chunk_results)
            }));
        }

        for handle in handles {
            for (source, html) in
                handle.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))?
            {
                rendered.insert(source, html);
            }
        }

        Ok(())
    })?;

    Ok(rendered)
}

fn render_mermaid_block_with<F>(
    content: &str,
    mermaid_mode: MermaidMode,
    render_mermaid: F,
) -> Result<String>
where
    F: Fn(&str) -> Result<String>,
{
    match mermaid_mode {
        MermaidMode::Disabled => {
            Ok("<div class=\"mdv-mermaid mdv-mermaid-fallback\">Mermaid disabled</div>".to_string())
        }
        MermaidMode::Enabled => match render_mermaid(content) {
            Ok(svg_markup) => Ok(format!(
                "<div class=\"mdv-mermaid\">{}</div>",
                decorate_mermaid_svg(&svg_markup)
            )),
            Err(_) => {
                Ok("<div class=\"mdv-mermaid mdv-mermaid-fallback\">Mermaid unavailable</div>"
                    .to_string())
            }
        },
    }
}

pub(super) fn decorate_mermaid_svg(svg_markup: &str) -> String {
    let mut sanitized = svg_markup.trim().to_string();
    if let Some(stripped) = sanitized.strip_prefix("<?xml version=\"1.0\" encoding=\"UTF-8\"?>") {
        sanitized = stripped.trim_start().to_string();
    }
    if sanitized.starts_with("<!DOCTYPE")
        && let Some(end) = sanitized.find('>')
    {
        sanitized = sanitized[end + 1..].trim_start().to_string();
    }

    sanitized.replacen("<svg", r#"<svg class="mdv-mermaid-diagram""#, 1)
}
