use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

#[test]
fn cli_starts_with_a_real_file_path() {
    let file = match NamedTempFile::new() {
        Ok(file) => file,
        Err(error) => panic!("temp file should be created: {error}"),
    };
    if let Err(error) = std::fs::write(file.path(), "# sample\n") {
        panic!("sample markdown should be written: {error}");
    }

    let mut command = match Command::cargo_bin("mdv") {
        Ok(command) => command,
        Err(error) => panic!("binary should build: {error}"),
    };

    command
        .arg(file.path())
        .arg("--theme")
        .arg("dark")
        .assert()
        .success()
        .stdout(contains("theme=dark"))
        .stdout(contains("path="));
}
