use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

#[test]
fn headless_render_covers_core_mvp_blocks() {
    let file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => panic!("temp file should be created: {error}"),
    };
    let source = "# Title\n\nParagraph with a [link](https://example.com).\n\n> [!WARNING]\n> Be careful\n\n- [x] done\n- [ ] pending\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n![diagram](docs/missing.png)\n\n```rust\nfn main() {}\n```\n\n```mermaid\ngraph TD\n    A --> B\n```\n\nFootnote ref[^1].\n\n[^1]: detail\n";
    if let Err(error) = std::fs::write(file.path(), source) {
        panic!("fixture should be written: {error}");
    }

    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg(file.path())
        .env("MDV_MERMAID_CMD", file.path().with_extension("missing-mermaid-executable"))
        .timeout(std::time::Duration::from_secs(15))
        .assert()
        .success()
        .stdout(contains("# Title"))
        .stdout(contains("link <https://example.com>"))
        .stdout(contains("[Warning] Be careful"))
        .stdout(contains("[x] done"))
        .stdout(contains("| A | B |"))
        .stdout(contains("[Image missing: diagram"))
        .stdout(contains("```rust"))
        .stdout(contains("[Mermaid"))
        .stdout(contains("[^1] detail"));
}

#[test]
fn headless_render_supports_repo_rich_fixture() {
    let fixture =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("rich_markdown.md");

    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg(&fixture)
        .env("MDV_MERMAID_CMD", fixture.with_extension("missing-mermaid-executable"))
        .timeout(std::time::Duration::from_secs(15))
        .assert()
        .success()
        .stdout(contains("# Rich Markdown Fixture"))
        .stdout(contains("[Note]"))
        .stdout(contains("[Image: Fixture image"))
        .stdout(contains("[Mermaid"))
        .stdout(contains(
            "[^details] The fixture keeps every high-value Markdown block in one document.",
        ));
}

#[test]
fn headless_render_supports_stdin_with_dash_path() {
    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg("-")
        .write_stdin("# Title\n\nParagraph with <br/> inline HTML.\n")
        .assert()
        .success()
        .stdout(contains("# Title"))
        .stdout(contains("Paragraph with <br/> inline HTML."));
}

#[test]
fn headless_render_strips_terminal_controls_from_file_and_stdin_input() {
    let source = "# visible\u{1b}]8;;https://example.com\u{1b}\\\u{1b}[31m\u{9b}31m\u{7}\u{8}\r\n\n\tparagraph\u{1b}[2J\n\n```rust\u{1b}[31m\nvalue\tkept\u{1b}]8;;https://example.com\u{7}x\u{1b}\\\u{9b}31m\u{8}\r\n```\n";
    let file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => panic!("temp file should be created: {error}"),
    };
    if let Err(error) = std::fs::write(file.path(), source) {
        panic!("fixture should be written: {error}");
    }

    let mut file_command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };
    let file_output = file_command
        .arg(file.path())
        .output()
        .unwrap_or_else(|error| panic!("file rendering should run: {error}"));
    assert!(file_output.status.success(), "file rendering should succeed: {file_output:?}");
    assert_terminal_controls_are_stripped(&String::from_utf8_lossy(&file_output.stdout));

    let mut stdin_command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };
    let stdin_output = stdin_command
        .arg("-")
        .write_stdin(source)
        .output()
        .unwrap_or_else(|error| panic!("stdin rendering should run: {error}"));
    assert!(stdin_output.status.success(), "stdin rendering should succeed: {stdin_output:?}");
    assert_terminal_controls_are_stripped(&String::from_utf8_lossy(&stdin_output.stdout));
}

#[test]
fn headless_render_keeps_math_expressions_visible() {
    let file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => panic!("temp file should be created: {error}"),
    };
    let source = "Inline $e^{i\\pi} + 1 = 0$ and $$a^2 + b^2 = c^2$$.\n";
    if let Err(error) = std::fs::write(file.path(), source) {
        panic!("fixture should be written: {error}");
    }

    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg(file.path())
        .assert()
        .success()
        .stdout(contains("$e^{i\\pi} + 1 = 0$"))
        .stdout(contains("$$a^2 + b^2 = c^2$$"));
}

fn assert_terminal_controls_are_stripped(output: &str) {
    assert!(output.contains("visible"), "safe document text should remain visible: {output:?}");
    assert!(output.contains("value\tkept"), "plain-text tabs should be preserved: {output:?}");
    assert!(output.contains('\n'), "plain-text newlines should be preserved: {output:?}");
    assert!(
        output.chars().all(|character| !character.is_control() || matches!(character, '\n' | '\t')),
        "headless output must not include terminal control characters: {output:?}"
    );
}
