use mdv::io::browser::browser_command_for;

#[test]
fn chooses_open_on_macos() {
    let command = browser_command_for("macos", "https://example.com");
    assert_eq!(command, Some(("open".to_string(), vec!["https://example.com".to_string()])));
}

#[test]
fn chooses_xdg_open_on_linux() {
    let command = browser_command_for("linux", "https://example.com");
    assert_eq!(command, Some(("xdg-open".to_string(), vec!["https://example.com".to_string()])));
}

#[test]
fn keeps_a_literal_url_as_one_unix_browser_argument() {
    let url = "https://example.com/search?q=a%20b&next=$(not-a-command)";
    let command = browser_command_for("linux", url);

    assert_eq!(command, Some(("xdg-open".to_string(), vec![url.to_string()])));
}
