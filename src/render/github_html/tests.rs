use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::core::config::MermaidMode;

use super::mermaid::{decorate_mermaid_svg, render_mermaid_blocks_with};

#[test]
fn decorate_mermaid_svg_adds_renderer_class() {
    let decorated = decorate_mermaid_svg(r#"<svg viewBox="0 0 10 10"><rect /></svg>"#);

    assert!(decorated.starts_with(r#"<svg class="mdv-mermaid-diagram""#));
    assert!(decorated.contains(r#"viewBox="0 0 10 10""#));
}

#[test]
fn render_mermaid_blocks_reuses_duplicate_sources() {
    let calls = Arc::new(AtomicUsize::new(0));
    let call_counter = Arc::clone(&calls);
    let sources =
        vec!["graph TD\n    A --> B\n".to_string(), "graph TD\n    A --> B\n".to_string()];

    let rendered = render_mermaid_blocks_with(&sources, MermaidMode::Enabled, move |content| {
        call_counter.fetch_add(1, Ordering::SeqCst);
        Ok(format!("<svg>{}</svg>", content.trim()))
    })
    .unwrap_or_else(|error| panic!("render should succeed: {error}"));

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(rendered.len(), 1);
    assert!(rendered.values().next().is_some_and(|html| html.contains("mdv-mermaid")));
}
