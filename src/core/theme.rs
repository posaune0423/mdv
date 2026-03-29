use crossterm::style::Color;

use crate::cli::Theme;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemeTokens {
    pub foreground: Color,
    pub muted: Color,
    pub accent: Color,
    pub subtle_background: Color,
    pub code_background: Color,
    pub warning: Color,
    pub status_background: Color,
}

impl ThemeTokens {
    #[must_use]
    pub const fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Light => Self {
                foreground: Color::Black,
                muted: Color::DarkGrey,
                accent: Color::DarkBlue,
                subtle_background: Color::White,
                code_background: Color::Grey,
                warning: Color::DarkYellow,
                status_background: Color::Grey,
            },
            Theme::Dark => Self {
                foreground: Color::White,
                muted: Color::Grey,
                accent: Color::Cyan,
                subtle_background: Color::Black,
                code_background: Color::DarkGrey,
                warning: Color::Yellow,
                status_background: Color::DarkGrey,
            },
        }
    }
}
