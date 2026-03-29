use mdv::{
    cli::Theme,
    render::{
        svg::render_viewport_png,
        text::{RenderedLine, RenderedLineKind},
    },
};

#[test]
fn viewport_svg_renderer_produces_png_bytes() {
    let lines = vec![
        RenderedLine {
            plain_text: "Title".to_string(),
            display_text: "Title".to_string(),
            kind: RenderedLineKind::Heading { level: 1 },
        },
        RenderedLine {
            plain_text: "Paragraph".to_string(),
            display_text: "Paragraph".to_string(),
            kind: RenderedLineKind::Paragraph,
        },
    ];

    let png = render_viewport_png(&lines, Theme::Light, 80, 10)
        .unwrap_or_else(|error| panic!("svg viewport should render: {error}"));

    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
}

#[test]
fn viewport_svg_renderer_changes_output_between_themes() {
    let lines = vec![
        RenderedLine {
            plain_text: "Title".to_string(),
            display_text: "Title".to_string(),
            kind: RenderedLineKind::Heading { level: 1 },
        },
        RenderedLine {
            plain_text: "Quoted".to_string(),
            display_text: "│ Quoted".to_string(),
            kind: RenderedLineKind::Quote,
        },
    ];

    let light = render_viewport_png(&lines, Theme::Light, 80, 10)
        .unwrap_or_else(|error| panic!("light svg viewport should render: {error}"));
    let dark = render_viewport_png(&lines, Theme::Dark, 80, 10)
        .unwrap_or_else(|error| panic!("dark svg viewport should render: {error}"));

    assert_ne!(light, dark);
}
