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
        .stdout(contains("--no-mermaid"))
        .stdout(contains("system"))
        .stdout(contains("update"));
}

#[test]
fn update_help_mentions_latest_release_install() {
    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(contains("latest GitHub Release"))
        .stdout(contains("current mdv executable"));
}
