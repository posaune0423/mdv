use mdv::{cli::Theme, core::config::MermaidMode, render::github_html::build_github_html};

#[test]
fn github_html_wraps_markdown_in_dark_github_shell() {
    let source = "# Title\n\nParagraph with *italic* and **bold**.\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("markdown-body"));
    assert!(html.contains("data-color-mode=\"dark\""));
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<em>italic</em>"));
    assert!(html.contains("<strong>bold</strong>"));
}

#[test]
fn github_html_replaces_mermaid_blocks_with_fallback_when_disabled() {
    let source = "```mermaid\ngraph TD\n    A --> B\n```\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("mdv-mermaid"));
    assert!(html.contains("Mermaid disabled"));
    assert!(!html.contains("<code class=\"language-mermaid\">"));
}

#[test]
fn github_html_injects_github_alert_icon_markup() {
    let source = "> [!NOTE]\n> Callout body.\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("markdown-alert markdown-alert-note"));
    assert!(html.contains("octicon octicon-info mr-2"));
    assert!(html.contains("Callout body."));
}

#[test]
fn github_html_uses_github_caution_icon_markup() {
    let source = "> [!CAUTION]\n> Callout body.\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("markdown-alert markdown-alert-caution"));
    assert!(html.contains("octicon octicon-stop mr-2"));
    assert!(html.contains(r#"d="M4.47.22A.749.749 0 0 1 5 0h6"#));
}

#[test]
fn github_html_keeps_relative_image_reference_for_base_url_resolution() {
    let source = "![Fixture image](pixel.png \"PNG sample image\")\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("src=\"pixel.png\""));
    assert!(html.contains("title=\"PNG sample image\""));
}

#[test]
fn github_html_uses_github_task_list_classes() {
    let source = "- [x] done\n- [ ] todo\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("class=\"contains-task-list\""));
    assert!(html.contains("class=\"task-list-item\""));
    assert!(html.contains("class=\"task-list-item-checkbox\""));
}

#[test]
fn github_html_applies_syntax_highlighting_to_code_fences() {
    let source = "```rust\nfn main() {}\n```\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains(
        "class=\"highlight highlight-source-rust notranslate position-relative overflow-auto\""
    ));
    assert!(html.contains("<pre"));
    assert!(html.contains("<span style="));
    assert!(html.contains("background-color: #161b22 !important;"));
}

#[test]
fn github_html_uses_dedicated_light_styles_without_theme_media_switching() {
    let source = "# Title\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Light, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("background: #ffffff;"));
    assert!(!html.contains("prefers-color-scheme"));
}

#[test]
fn github_html_uses_github_dark_token_colors_for_code_fences() {
    let source = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("color:#ff7b72"));
    assert!(html.contains("color:#79c0ff"));
}

#[test]
fn github_html_embeds_github_font_faces_for_snapshot_rendering() {
    let html = build_github_html(
        "# Title\n",
        std::path::Path::new("."),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("font-family: 'Mona Sans VF'"));
    assert!(html.contains("data:font/woff2;base64,"));
}

#[test]
fn github_html_uses_embedded_github_font_stacks() {
    let html = build_github_html(
        "# Title\n\nParagraph with `code`.\n",
        std::path::Path::new("."),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains(
        r#"font-family: 'Mona Sans VF', -apple-system, system-ui, "Segoe UI", "Noto Sans""#
    ));
    assert!(html.contains(
        r#"font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono","#
    ));
    assert!(html.contains(r#"src: url('data:font/woff2;base64,"#));
    assert!(html.contains(r#"format('woff2')"#));
}

#[test]
fn github_html_escapes_untrusted_raw_html() {
    let html = build_github_html(
        "<script>window.evil = true</script>\n",
        std::path::Path::new("."),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(!html.contains("<script>window.evil = true</script>"));
    assert!(html.contains("&lt;script&gt;window.evil = true&lt;/script&gt;"));
}

#[test]
fn github_html_preserves_mermaid_inside_list_structure() {
    let source = "- before\n  ```mermaid\n  graph TD\n      A --> B\n  ```\n- after\n";

    let html =
        build_github_html(source, std::path::Path::new("."), Theme::Dark, MermaidMode::Disabled)
            .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("<li>"));
    assert!(html.contains("mdv-mermaid"));
    assert!(html.contains("before"));
    assert!(html.contains("after"));
}

#[test]
fn github_html_does_not_lock_prose_weight_to_400() {
    let html = build_github_html(
        "# Title\n\nParagraph with **bold**.\n",
        std::path::Path::new("."),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(!html.contains(r#"font-variation-settings: "wght" 400"#));
}

#[test]
fn github_html_does_not_force_antialiased_text_smoothing() {
    let html = build_github_html(
        "# Title\n\nParagraph.\n",
        std::path::Path::new("."),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(!html.contains("-webkit-font-smoothing: antialiased;"));
    assert!(!html.contains("text-rendering: optimizeLegibility;"));
}

#[test]
fn github_html_uses_viewport_relative_article_width() {
    let html = build_github_html(
        "# Title\n",
        std::path::Path::new("."),
        Theme::Dark,
        MermaidMode::Disabled,
    )
    .unwrap_or_else(|error| panic!("html should render: {error}"));

    assert!(html.contains("width: calc(100vw - 16px);"));
    assert!(html.contains("max-width: none;"));
}
