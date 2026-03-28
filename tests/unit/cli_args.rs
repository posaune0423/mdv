use mdv::cli::{MdvArgs, Theme};

#[test]
fn parses_watch_theme_and_mermaid_flags() {
    let args =
        MdvArgs::parse_from(["mdv", "docs/PRD.md", "--watch", "--theme", "dark", "--no-mermaid"]);

    assert_eq!(args.theme, Theme::Dark);
    assert!(args.watch);
    assert!(args.no_mermaid);
    assert_eq!(args.path.to_string_lossy(), "docs/PRD.md");
}
