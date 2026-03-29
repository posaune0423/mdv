use mdv::cli::{MdvArgs, MdvCommand, Theme};

#[test]
fn parses_watch_theme_and_mermaid_flags() {
    let args =
        MdvArgs::parse_from(["mdv", "docs/PRD.md", "--watch", "--theme", "dark", "--no-mermaid"]);

    assert_eq!(args.theme, Theme::Dark);
    assert!(args.watch);
    assert!(args.no_mermaid);
    assert_eq!(args.path.as_deref(), Some(std::path::Path::new("docs/PRD.md")));
}

#[test]
fn parses_update_subcommand_without_document_path() {
    let args = MdvArgs::parse_from(["mdv", "update"]);

    assert_eq!(args.command, Some(MdvCommand::Update));
    assert_eq!(args.path, None);
}

#[test]
fn defaults_theme_to_system() {
    let args = MdvArgs::parse_from(["mdv", "docs/PRD.md"]);

    assert_eq!(args.theme, Theme::System);
}
