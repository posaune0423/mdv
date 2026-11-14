use mdv::io::browser::browser_command_for;

#[test]
fn chooses_open_on_macos() {
    let command = browser_command_for("macos", "https://example.com");
    assert_eq!(
        command,
        Some(("open".to_string(), vec!["--".to_string(), "https://example.com/".to_string()]))
    );
}

#[test]
fn chooses_xdg_open_on_linux() {
    let command = browser_command_for("linux", "https://example.com");
    assert_eq!(command, Some(("xdg-open".to_string(), vec!["https://example.com/".to_string()])));
}

#[test]
fn normalizes_a_valid_web_url_before_passing_it_to_the_browser() {
    let command = browser_command_for("linux", " HTTPS://EXAMPLE.COM:443/path ");

    assert_eq!(
        command,
        Some(("xdg-open".to_string(), vec!["https://example.com/path".to_string()]))
    );
}

#[test]
fn allows_mailto_targets_on_unix_platforms() {
    let command = browser_command_for("linux", "mailto:reader@example.com");

    assert_eq!(
        command,
        Some(("xdg-open".to_string(), vec!["mailto:reader@example.com".to_string()]))
    );
}

#[test]
fn rejects_local_paths_custom_schemes_and_command_options_on_unix_platforms() {
    for target in [
        "./relative-link.md",
        "/tmp/local-link.md",
        "file:///tmp/local-link.md",
        "shell:AppsFolder\\cmd.exe",
        "ftp://example.com/archive",
        "--new-window",
    ] {
        assert_eq!(browser_command_for("linux", target), None, "target: {target}");
    }
}
