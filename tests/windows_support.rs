use mdv::ui::terminal::is_supported_terminal;

#[test]
fn wezterm_is_an_interactive_image_terminal() {
    assert!(is_supported_terminal(Some("WezTerm"), Some("xterm-256color")));
    assert!(is_supported_terminal(Some("wezterm"), None));
}

#[test]
fn windows_terminal_is_not_mistaken_for_a_lossless_image_terminal() {
    assert!(!is_supported_terminal(Some("Windows Terminal"), Some("xterm-256color")));
}
