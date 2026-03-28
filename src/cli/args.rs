use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Clone, Debug, Eq, Parser, PartialEq)]
#[command(name = "mdv", version, about = "High-fidelity Markdown viewer for Ghostty and Kitty")]
pub struct MdvArgs {
    /// Markdown file to open.
    pub path: PathBuf,

    /// Reload the document when the file changes.
    #[arg(long)]
    pub watch: bool,

    /// Color theme used by the viewer.
    #[arg(long, value_enum, default_value_t = Theme::Light)]
    pub theme: Theme,

    /// Disable Mermaid rendering and show placeholders instead.
    #[arg(long = "no-mermaid")]
    pub no_mermaid: bool,
}

impl MdvArgs {
    #[must_use]
    pub fn parse_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        <Self as Parser>::parse_from(itr)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}
