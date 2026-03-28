#![forbid(unsafe_code)]

pub mod cli;

#[cfg(test)]
mod tests {
    use crate::cli::{MdvArgs, Theme};

    #[test]
    fn default_theme_is_light() {
        let args = MdvArgs::parse_from(["mdv", "README.md"]);
        assert_eq!(args.theme, Theme::Light);
    }
}
