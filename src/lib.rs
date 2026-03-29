#![forbid(unsafe_code)]

pub mod app;
pub mod cli;
pub mod core;
pub mod io;
pub mod render;
pub mod support;
pub mod ui;

#[cfg(test)]
mod tests {
    use crate::cli::{MdvArgs, Theme};

    #[test]
    fn default_theme_is_system() {
        let args = MdvArgs::parse_from(["mdv", "README.md"]);
        assert_eq!(args.theme, Theme::System);
    }
}
