use std::sync::{Arc, OnceLock};

use anyhow::{Result, anyhow};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg::{self, Options},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme as SyntectTheme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

use crate::{
    cli::Theme,
    core::document::CalloutKind,
    render::text::{RenderedInlineSegment, RenderedLine, RenderedLineKind},
};

const CELL_WIDTH_PX: u32 = 10;
const LINE_HEIGHT_PX: u32 = 28;
const BASELINE_OFFSET_PX: u32 = 20;
const MONO_ADVANCE_PX: u32 = 9;
const BODY_FONT: &str = if cfg!(target_os = "macos") { "Helvetica" } else { "DejaVu Sans" };
const MONO_FONT: &str = if cfg!(target_os = "macos") { "Menlo" } else { "DejaVu Sans Mono" };

static FONT_DATABASE: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
static FALLBACK_THEME: OnceLock<SyntectTheme> = OnceLock::new();

pub fn render_viewport_png(
    lines: &[RenderedLine],
    theme: Theme,
    width_cells: u16,
    height_rows: u16,
) -> Result<Vec<u8>> {
    let width_px = u32::from(width_cells.max(1)) * CELL_WIDTH_PX;
    let height_px = u32::from(height_rows.max(1)) * LINE_HEIGHT_PX;
    let svg = build_svg(lines, theme, width_px, height_px);

    let options = Options {
        font_family: BODY_FONT.to_string(),
        fontdb: font_database(),
        ..Options::default()
    };
    let tree = usvg::Tree::from_str(&svg, &options)?;
    let mut pixmap = Pixmap::new(width_px, height_px)
        .ok_or_else(|| anyhow!("failed to allocate viewport pixmap"))?;
    resvg::render(&tree, Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(Into::into)
}

fn build_svg(lines: &[RenderedLine], theme: Theme, width_px: u32, height_px: u32) -> String {
    let palette = match theme {
        Theme::Light | Theme::System => Palette {
            page_background: "#f6f8fa",
            article_background: "#ffffff",
            foreground: "#1f2328",
            muted: "#57606a",
            border: "#d0d7de",
            code_background: "#f6f8fa",
            table_header_background: "#f6f8fa",
            callout_background: "#ddf4ff",
            callout_border: "#0969da",
            warning_background: "#fff8c5",
            warning_border: "#9a6700",
            quote_border: "#d0d7de",
        },
        Theme::Dark => Palette {
            page_background: "#0d1117",
            article_background: "#161b22",
            foreground: "#e6edf3",
            muted: "#8b949e",
            border: "#30363d",
            code_background: "#0d1117",
            table_header_background: "#1f2630",
            callout_background: "#0c2d6b",
            callout_border: "#2f81f7",
            warning_background: "#3b2f13",
            warning_border: "#d29922",
            quote_border: "#30363d",
        },
    };
    let available_width = width_px.saturating_sub(40);
    let article_width =
        if available_width < 320 { available_width.max(1) } else { available_width.min(980) };
    let article_x = width_px.saturating_sub(article_width) / 2;
    let article_y = 14;
    let article_height = height_px.saturating_sub(24).max(LINE_HEIGHT_PX);
    let content_x = article_x + 40;
    let content_width = article_width.saturating_sub(80);

    let mut body = String::new();
    body.push_str(&format!(
        r#"<rect x="0" y="0" width="{width_px}" height="{height_px}" fill="{}"/>"#,
        palette.page_background
    ));
    body.push_str(&format!(
        r#"<rect x="{article_x}" y="{article_y}" width="{article_width}" height="{article_height}" rx="14" fill="{}" stroke="{}" stroke-width="1"/>"#,
        palette.article_background, palette.border
    ));

    for (index, line) in lines.iter().enumerate() {
        let y = article_y + 14 + index as u32 * LINE_HEIGHT_PX;
        let baseline = y + BASELINE_OFFSET_PX;
        match &line.kind {
            RenderedLineKind::Blank => {}
            RenderedLineKind::Heading { level } => {
                body.push_str(&svg_text(
                    &line.display_text,
                    content_x,
                    baseline,
                    palette.foreground,
                    match *level {
                        1 => 34,
                        2 => 28,
                        3 => 22,
                        _ => 19,
                    },
                    700,
                    BODY_FONT,
                ));
                if *level <= 2 {
                    let rule_y = y + LINE_HEIGHT_PX - 2;
                    body.push_str(&format!(
                        r#"<line x1="{content_x}" y1="{rule_y}" x2="{}" y2="{rule_y}" stroke="{}" stroke-width="1"/>"#,
                        content_x + content_width,
                        palette.border
                    ));
                }
            }
            RenderedLineKind::Paragraph | RenderedLineKind::List => {
                body.push_str(&svg_rich_text(
                    &line.spans,
                    content_x,
                    baseline,
                    palette.foreground,
                    17,
                ));
            }
            RenderedLineKind::Quote => {
                body.push_str(&format!(
                    r#"<rect x="{}" y="{y}" width="4" height="{LINE_HEIGHT_PX}" rx="2" fill="{}"/>"#,
                    content_x.saturating_sub(18),
                    palette.quote_border
                ));
                body.push_str(&svg_rich_text(&line.spans, content_x, baseline, palette.muted, 16));
            }
            RenderedLineKind::Callout { kind } => {
                let (fill, border, text_color) =
                    if matches!(*kind, CalloutKind::Warning | CalloutKind::Caution) {
                        (palette.warning_background, palette.warning_border, palette.foreground)
                    } else {
                        (palette.callout_background, palette.callout_border, palette.foreground)
                    };
                body.push_str(&format!(
                    r#"<rect x="{}" y="{y}" width="{content_width}" height="{LINE_HEIGHT_PX}" rx="8" fill="{fill}" stroke="{border}" stroke-width="1"/>"#,
                    content_x.saturating_sub(14),
                ));
                body.push_str(&format!(
                    r#"<rect x="{}" y="{y}" width="4" height="{LINE_HEIGHT_PX}" rx="2" fill="{border}"/>"#,
                    content_x.saturating_sub(14),
                ));
                body.push_str(&svg_rich_text(&line.spans, content_x + 4, baseline, text_color, 16));
            }
            RenderedLineKind::Code { language, is_fence_delimiter } => {
                body.push_str(&format!(
                    r#"<rect x="{}" y="{y}" width="{content_width}" height="{LINE_HEIGHT_PX}" rx="6" fill="{}" stroke="{}" stroke-width="1"/>"#,
                    content_x.saturating_sub(14),
                    palette.code_background,
                    palette.border
                ));
                if *is_fence_delimiter {
                    body.push_str(&svg_text(
                        &line.display_text,
                        content_x + 6,
                        baseline,
                        palette.muted,
                        15,
                        500,
                        MONO_FONT,
                    ));
                } else {
                    body.push_str(&svg_highlighted_code(
                        &line.plain_text,
                        language.as_deref(),
                        theme,
                        content_x + 6,
                        baseline,
                    ));
                }
            }
            RenderedLineKind::Table => {
                let fill = if index % 2 == 0 {
                    palette.table_header_background
                } else {
                    palette.article_background
                };
                body.push_str(&format!(
                    r#"<rect x="{}" y="{y}" width="{content_width}" height="{LINE_HEIGHT_PX}" fill="{fill}" stroke="{}" stroke-width="1"/>"#,
                    content_x.saturating_sub(14),
                    palette.border
                ));
                body.push_str(&svg_text(
                    &line.display_text,
                    content_x + 4,
                    baseline,
                    palette.foreground,
                    15,
                    400,
                    MONO_FONT,
                ));
            }
            RenderedLineKind::Rule => {
                let mid_y = y + (LINE_HEIGHT_PX / 2);
                body.push_str(&format!(
                    r#"<line x1="{content_x}" y1="{mid_y}" x2="{}" y2="{mid_y}" stroke="{}" stroke-width="1"/>"#,
                    content_x + content_width,
                    palette.border
                ));
            }
            RenderedLineKind::Meta => {
                body.push_str(&svg_rich_text(&line.spans, content_x, baseline, palette.muted, 14));
            }
        }
    }

    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width_px}" height="{height_px}" viewBox="0 0 {width_px} {height_px}">{body}</svg>"#
    )
}

struct Palette {
    page_background: &'static str,
    article_background: &'static str,
    foreground: &'static str,
    muted: &'static str,
    border: &'static str,
    code_background: &'static str,
    table_header_background: &'static str,
    callout_background: &'static str,
    callout_border: &'static str,
    warning_background: &'static str,
    warning_border: &'static str,
    quote_border: &'static str,
}

fn svg_text(
    text: &str,
    x: u32,
    y: u32,
    fill: &str,
    font_size: u32,
    font_weight: u32,
    font_family: &str,
) -> String {
    format!(
        r#"<text x="{x}" y="{y}" fill="{fill}" font-size="{font_size}" font-weight="{font_weight}" font-family="{font_family}">{}</text>"#,
        escape_xml(text)
    )
}

fn svg_rich_text(
    spans: &[RenderedInlineSegment],
    x: u32,
    y: u32,
    fill: &str,
    font_size: u32,
) -> String {
    if spans.is_empty() {
        return String::new();
    }

    let segments = spans
        .iter()
        .filter(|segment| !segment.text.is_empty())
        .map(|segment| {
            let font_family = if segment.style.code { MONO_FONT } else { BODY_FONT };
            let font_weight = if segment.style.bold { 600 } else { 400 };
            let font_style = if segment.style.italic { "italic" } else { "normal" };
            format!(
                r#"<tspan font-family="{font_family}" font-weight="{font_weight}" font-style="{font_style}">{}</tspan>"#,
                escape_xml(&segment.text)
            )
        })
        .collect::<String>();

    format!(
        r#"<text x="{x}" y="{y}" fill="{fill}" font-size="{font_size}" xml:space="preserve">{segments}</text>"#
    )
}

fn svg_highlighted_code(
    text: &str,
    language: Option<&str>,
    theme: Theme,
    x: u32,
    y: u32,
) -> String {
    if text.is_empty() {
        return String::new();
    }

    let syntax_set = syntax_set();
    let syntax = syntax_for_token(syntax_set, language);
    let mut highlighter = HighlightLines::new(syntax, syntect_theme(theme));
    let mut cursor_x = x;
    let mut svg = String::new();

    if let Ok(segments) = highlighter.highlight_line(text, syntax_set) {
        for (style, piece) in segments {
            if piece.is_empty() {
                continue;
            }
            svg.push_str(&svg_text(
                piece,
                cursor_x,
                y,
                &syntect_color_hex(style),
                15,
                400,
                MONO_FONT,
            ));
            cursor_x = cursor_x.saturating_add(mono_advance(piece));
        }
    }

    svg
}

fn mono_advance(text: &str) -> u32 {
    text.chars().count() as u32 * MONO_ADVANCE_PX
}

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn font_database() -> Arc<usvg::fontdb::Database> {
    FONT_DATABASE
        .get_or_init(|| {
            let mut fontdb = usvg::fontdb::Database::new();
            for path in preferred_font_paths() {
                let _ = fontdb.load_font_file(path);
            }
            if fontdb.faces().next().is_none() {
                fontdb.load_system_fonts();
            }
            Arc::new(fontdb)
        })
        .clone()
}

fn syntax_for_token<'a>(syntax_set: &'a SyntaxSet, language: Option<&str>) -> &'a SyntaxReference {
    language
        .and_then(|token| syntax_set.find_syntax_by_token(token))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
}

fn syntect_theme(theme: Theme) -> &'static SyntectTheme {
    let themes = THEME_SET.get_or_init(ThemeSet::load_defaults);
    let preferred = match theme {
        Theme::Light | Theme::System => "InspiredGitHub",
        Theme::Dark => "base16-ocean.dark",
    };

    themes
        .themes
        .get(preferred)
        .or_else(|| themes.themes.values().next())
        .unwrap_or_else(|| FALLBACK_THEME.get_or_init(SyntectTheme::default))
}

fn syntect_color_hex(style: Style) -> String {
    format!("#{:02x}{:02x}{:02x}", style.foreground.r, style.foreground.g, style.foreground.b)
}

fn preferred_font_paths() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/Menlo.ttc",
            "/System/Library/Fonts/Supplemental/Helvetica.ttc",
            "/System/Library/Fonts/Supplemental/Menlo.ttc",
        ]
    } else {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
        ]
    }
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::MONO_FONT;
    use crate::{
        cli::Theme,
        render::text::{
            RenderedInlineSegment, RenderedInlineStyle, RenderedLine, RenderedLineKind,
        },
    };

    #[test]
    fn build_svg_preserves_inline_emphasis_weight_and_style() {
        let svg = super::build_svg(
            &[RenderedLine {
                plain_text: "bold italic code".to_string(),
                display_text: "bold italic code".to_string(),
                spans: vec![
                    RenderedInlineSegment {
                        text: "bold".to_string(),
                        style: RenderedInlineStyle::BOLD,
                    },
                    RenderedInlineSegment {
                        text: " ".to_string(),
                        style: RenderedInlineStyle::PLAIN,
                    },
                    RenderedInlineSegment {
                        text: "italic".to_string(),
                        style: RenderedInlineStyle::ITALIC,
                    },
                    RenderedInlineSegment {
                        text: " ".to_string(),
                        style: RenderedInlineStyle::PLAIN,
                    },
                    RenderedInlineSegment {
                        text: "code".to_string(),
                        style: RenderedInlineStyle::CODE,
                    },
                ],
                kind: RenderedLineKind::Paragraph,
            }],
            Theme::Light,
            800,
            120,
        );

        assert!(svg.contains(r#"font-weight="600""#), "{svg}");
        assert!(svg.contains(r#"font-style="italic""#), "{svg}");
        assert!(svg.contains(&format!("font-family=\"{MONO_FONT}\"")), "{svg}");
    }
}
