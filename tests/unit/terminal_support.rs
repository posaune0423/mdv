use mdv::ui::terminal::{
    is_supported_terminal, is_supported_tmux_outer_terminal, known_tmux_outer_terminal_from,
};

#[test]
fn accepts_ghostty_and_kitty() {
    assert!(is_supported_terminal(Some("ghostty"), None));
    assert!(is_supported_terminal(None, Some("xterm-kitty")));
}

#[test]
fn rejects_unknown_terminal() {
    assert!(!is_supported_terminal(Some("iterm"), Some("xterm-256color")));
}

#[test]
fn recognizes_a_ghostty_or_kitty_outer_terminal_in_tmux() {
    assert!(is_supported_tmux_outer_terminal(Some("ghostty")));
    assert!(is_supported_tmux_outer_terminal(Some("kitty")));
    assert!(!is_supported_tmux_outer_terminal(Some("wezterm")));
}

#[test]
fn only_uses_lc_terminal_as_a_tmux_fallback() {
    assert!(known_tmux_outer_terminal_from(true, Some("ghostty"), Ok(false)));
    assert!(known_tmux_outer_terminal_from(true, None, Ok(true)));
    assert!(!known_tmux_outer_terminal_from(false, Some("ghostty"), Ok(false)));
}
