use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

#[test]
fn prints_rendered_document_when_stdout_is_not_a_tty() {
    let file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => panic!("temp file should be created: {error}"),
    };
    if let Err(error) = std::fs::write(
        file.path(),
        "# Title\n\nParagraph text.\n\n```mermaid\ngraph TD\n    A --> B\n```\n",
    ) {
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
        .stdout(contains("Title"))
        .stdout(contains("Paragraph text."))
        .stdout(contains("[Mermaid"));
}

#[test]
fn prints_disabled_mermaid_placeholder_when_flag_is_set() {
    let file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => panic!("temp file should be created: {error}"),
    };
    if let Err(error) = std::fs::write(file.path(), "```mermaid\ngraph TD\n    A --> B\n```\n") {
        panic!("fixture should be written: {error}");
    }

    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg(file.path())
        .arg("--no-mermaid")
        .assert()
        .success()
        .stdout(contains("[Mermaid disabled]"));
}
