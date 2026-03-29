use std::sync::OnceLock;

use syntect::{
    easy::HighlightLines,
    highlighting::{Theme as SyntectTheme, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
    util::as_24_bit_terminal_escaped,
};

use crate::cli::Theme;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
static FALLBACK_THEME: OnceLock<SyntectTheme> = OnceLock::new();

pub(super) fn highlight_code_terminal(text: &str, language: Option<&str>, theme: Theme) -> String {
    if text.is_empty() {
        return String::new();
    }

    let syntax_set = syntax_set();
    let syntax = syntax_for_token(syntax_set, language);
    let mut highlighter = HighlightLines::new(syntax, syntect_theme(theme));
    highlighter
        .highlight_line(text, syntax_set)
        .map(|segments| as_24_bit_terminal_escaped(&segments[..], false))
        .unwrap_or_else(|_| text.to_string())
}

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syntax_for_token<'a>(syntax_set: &'a SyntaxSet, language: Option<&str>) -> &'a SyntaxReference {
    language
        .and_then(|token| syntax_set.find_syntax_by_token(token))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
}

fn syntect_theme(theme: Theme) -> &'static SyntectTheme {
    let themes = THEME_SET.get_or_init(ThemeSet::load_defaults);
    let preferred = match theme {
        Theme::Light => "InspiredGitHub",
        Theme::Dark => "base16-ocean.dark",
    };

    themes
        .themes
        .get(preferred)
        .or_else(|| themes.themes.values().next())
        .unwrap_or_else(|| FALLBACK_THEME.get_or_init(SyntectTheme::default))
}
