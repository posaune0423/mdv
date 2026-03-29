use std::{path::Path, sync::OnceLock};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use comrak::plugins::syntect::{SyntectAdapter, SyntectAdapterBuilder};

use crate::cli::Theme;

static LIGHT_SYNTAX_ADAPTER: OnceLock<SyntectAdapter> = OnceLock::new();
static DARK_SYNTAX_ADAPTER: OnceLock<SyntectAdapter> = OnceLock::new();

pub(super) fn theme_styles(theme: Theme) -> String {
    let font_faces = font_face_css();
    match theme {
        Theme::Light | Theme::System => [
            font_faces.as_str(),
            "\n",
            LIGHT_CSS,
            "\n",
            COMMON_OVERRIDES,
            "\n",
            LIGHT_THEME_OVERRIDES,
        ]
        .concat(),
        Theme::Dark => [
            font_faces.as_str(),
            "\n",
            DARK_CSS,
            "\n",
            COMMON_OVERRIDES,
            "\n",
            DARK_THEME_OVERRIDES,
        ]
        .concat(),
    }
}

pub(super) fn syntax_adapter(theme: Theme) -> &'static SyntectAdapter {
    match theme {
        Theme::Light | Theme::System => LIGHT_SYNTAX_ADAPTER
            .get_or_init(|| SyntectAdapterBuilder::new().theme("InspiredGitHub").build()),
        Theme::Dark => DARK_SYNTAX_ADAPTER
            .get_or_init(|| SyntectAdapterBuilder::new().theme("base16-ocean.dark").build()),
    }
}

fn font_face_css() -> String {
    let mona_sans_url = font_data_url(include_bytes!("../assets/fonts/MonaSansVF.woff2"));

    format!(
        r#"
@font-face {{
  font-family: 'Mona Sans VF';
  src: url('{mona_sans_url}') format('woff2');
  font-weight: 200 900;
  font-stretch: 75% 125%;
  font-style: normal;
  font-optical-sizing: auto;
}}

@font-face {{
  font-family: 'Mona Sans VF';
  src: url('{mona_sans_url}') format('woff2');
  font-weight: 200 900;
  font-stretch: 75% 125%;
  font-style: italic;
  font-optical-sizing: auto;
}}
"#
    )
}

fn font_data_url(bytes: &[u8]) -> String {
    format!("data:font/woff2;base64,{}", STANDARD.encode(bytes))
}

fn path_url(path: &Path) -> String {
    let mut encoded = String::from("file://");
    let display = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    for byte in display.to_string_lossy().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub(super) fn directory_url(path: &Path) -> String {
    let mut encoded = path_url(path);
    if !encoded.ends_with('/') {
        encoded.push('/');
    }
    encoded
}

const LIGHT_CSS: &str = include_str!("../assets/github-markdown-light.css");

const COMMON_OVERRIDES: &str = r#"
body {
  margin: 0;
}
.mdv-page {
  padding: 8px;
  display: flex;
  justify-content: center;
}
.markdown-body {
  width: calc(100vw - 16px);
  max-width: none;
  background: transparent;
  font-family: 'Mona Sans VF', -apple-system, system-ui, "Segoe UI", "Noto Sans",
    Helvetica, Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji";
}
.markdown-body img {
  max-width: 100%;
  height: auto;
}
.markdown-body [align="center"] {
  text-align: center;
}
.markdown-body [align="left"] {
  text-align: left;
}
.markdown-body [align="right"] {
  text-align: right;
}
.markdown-body p > img:only-child {
  display: block;
}
.mdv-mermaid {
  margin: 16px 0;
  display: flex;
  justify-content: center;
  overflow: hidden;
  max-width: min(100%, 420px);
  margin-left: auto;
  margin-right: auto;
}
.mdv-mermaid img,
.mdv-mermaid-diagram {
  display: block;
  width: 100%;
  max-width: 100%;
  height: auto;
  margin: 0 auto;
  flex: 0 1 auto;
}
.mdv-mermaid-fallback {
  color: #57606a;
  font-weight: 600;
}
.markdown-body code,
.markdown-body tt,
.markdown-body pre,
.markdown-body kbd,
.markdown-body samp {
  font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono",
    monospace !important;
}
"#;

const LIGHT_THEME_OVERRIDES: &str = r#"
body {
  background: #ffffff;
}
.markdown-body .highlight,
.markdown-body pre {
  background-color: #f6f8fa !important;
  border: 1px solid #d0d7de;
  border-radius: 6px;
}
.markdown-body .highlight pre,
.markdown-body pre {
  padding: 16px;
}
.markdown-body pre code,
.markdown-body .highlight pre code {
  background: transparent !important;
}
"#;

const DARK_CSS: &str = include_str!("../assets/github-markdown-dark.css");

const DARK_THEME_OVERRIDES: &str = r#"
body {
  background: #0d1117;
}
.mdv-mermaid-fallback {
  color: #8b949e;
  font-weight: 600;
}
.markdown-body .highlight,
.markdown-body pre {
  background-color: #161b22 !important;
  border: 1px solid #30363d;
  border-radius: 6px;
}
.markdown-body .highlight pre,
.markdown-body pre {
  padding: 16px;
}
.markdown-body pre code,
.markdown-body .highlight pre code {
  background: transparent !important;
}
"#;
