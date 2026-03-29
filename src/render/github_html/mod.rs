use std::path::Path;

use anyhow::{Context, Result};
use comrak::{Arena, format_html_with_plugins, options::Plugins, parse_document};
use tracing::info_span;

use crate::{
    cli::Theme, core::config::MermaidMode, io::mermaid_cli::MermaidCliRenderer,
    render::markdown_pipeline::gfm_options,
};

mod mermaid;
mod postprocess;
mod styles;
#[cfg(test)]
mod tests;

use self::{
    mermaid::replace_mermaid_code_blocks,
    postprocess::{decorate_code_blocks, inject_alert_icons, retint_code_tokens},
    styles::{directory_url, syntax_adapter, theme_styles},
};

pub fn build_github_html(
    source: &str,
    base_dir: &Path,
    theme: Theme,
    mermaid_mode: MermaidMode,
) -> Result<String> {
    let _span = info_span!("github_html.build").entered();
    let mut options = gfm_options();
    options.render.escape = true;
    options.render.r#unsafe = false;
    options.render.tasklist_classes = true;
    options.render.github_pre_lang = true;

    let arena = Arena::new();
    let root = parse_document(&arena, source, &options);
    replace_mermaid_code_blocks(root, mermaid_mode, &MermaidCliRenderer::from_env())?;

    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(syntax_adapter(theme));

    let mut rendered = String::new();
    format_html_with_plugins(root, &options, &mut rendered, &plugins)
        .context("formatting markdown to html failed")?;

    let body_html =
        decorate_code_blocks(&retint_code_tokens(&inject_alert_icons(&rendered), theme));
    let styles = theme_styles(theme);

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en" data-color-mode="{color_mode}" data-dark-theme="dark" data-light-theme="light">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <base href="{base_href}" />
  <style>{styles}</style>
</head>
<body>
  <div class="mdv-page">
    <article class="markdown-body entry-content container-lg">
      {body_html}
    </article>
  </div>
</body>
</html>"#,
        color_mode = theme.as_str(),
        base_href = directory_url(base_dir),
        styles = styles,
    ))
}
