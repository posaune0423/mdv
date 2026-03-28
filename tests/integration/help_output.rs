use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn help_mentions_core_flags() {
    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("--watch"))
        .stdout(contains("--theme"))
        .stdout(contains("--no-mermaid"));
}
