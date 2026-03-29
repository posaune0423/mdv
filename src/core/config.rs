use std::path::PathBuf;

use crate::cli::{MdvArgs, Theme};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppConfig {
    pub path: PathBuf,
    pub watch: bool,
    pub theme: Theme,
    pub mermaid_mode: MermaidMode,
}

impl From<MdvArgs> for AppConfig {
    fn from(value: MdvArgs) -> Self {
        let mermaid_mode =
            if value.no_mermaid { MermaidMode::Disabled } else { MermaidMode::Enabled };

        Self { path: value.path, watch: value.watch, theme: value.theme, mermaid_mode }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MermaidMode {
    Enabled,
    Disabled,
}
