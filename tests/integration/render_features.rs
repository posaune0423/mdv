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
