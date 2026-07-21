use mdv::ui::terminal::is_supported_terminal;

#[test]
fn accepts_ghostty_and_kitty() {
    assert!(is_supported_terminal(Some("ghostty"), None, None));
    assert!(is_supported_terminal(None, None, Some("xterm-kitty")));
}

#[test]
fn falls_back_to_lc_terminal_inside_tmux() {
    assert!(is_supported_terminal(Some("tmux"), Some("ghostty"), Some("tmux-256color"),));
}

#[test]
fn rejects_unknown_terminal() {
    assert!(!is_supported_terminal(Some("tmux"), Some("iterm"), Some("tmux-256color"),));
}
