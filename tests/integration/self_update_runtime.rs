use std::{env, fs, path::Path, process::Command};

use mdv::io::self_update::replace_running_executable;

const REPLACEMENT_ENV: &str = "MDV_SELF_UPDATE_REPLACEMENT";
const EXPECT_FAILURE_ENV: &str = "MDV_SELF_UPDATE_EXPECT_FAILURE";

#[test]
fn replaces_a_running_child_executable() {
    let test_executable = env::current_exe()
        .unwrap_or_else(|error| panic!("the integration test executable should resolve: {error}"));
    let workspace = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("the replacement workspace should be created: {error}"));
    let helper = workspace.path().join("self-update-helper.exe");
    let replacement = workspace.path().join("replacement.exe");
    let replacement_bytes = b"mdv self-update replacement marker";

    fs::copy(&test_executable, &helper).unwrap_or_else(|error| {
        panic!("the test executable should be copied for replacement: {error}")
    });
    fs::write(&replacement, replacement_bytes)
        .unwrap_or_else(|error| panic!("the replacement marker should be written: {error}"));

    let output = Command::new(&helper)
        .args(["--ignored", "--exact", "integration::self_update_runtime::replaces_running_helper"])
        .env(REPLACEMENT_ENV, &replacement)
        .output()
        .unwrap_or_else(|error| panic!("the copied helper should run: {error}"));
    assert!(
        output.status.success(),
        "the helper should schedule its own replacement: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&helper).unwrap_or_else(|error| panic!("the helper should still exist: {error}")),
        replacement_bytes
    );
}

#[test]
fn restores_a_running_child_executable_when_replacement_cannot_be_copied() {
    let test_executable = env::current_exe()
        .unwrap_or_else(|error| panic!("the integration test executable should resolve: {error}"));
    let workspace = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("the replacement workspace should be created: {error}"));
    let helper = workspace.path().join("self-update-rollback-helper.exe");
    let missing_replacement = workspace.path().join("replacement-does-not-exist.exe");

    fs::copy(&test_executable, &helper).unwrap_or_else(|error| {
        panic!("the test executable should be copied for rollback: {error}")
    });

    let output = Command::new(&helper)
        .args(["--ignored", "--exact", "integration::self_update_runtime::replaces_running_helper"])
        .env(REPLACEMENT_ENV, &missing_replacement)
        .env(EXPECT_FAILURE_ENV, "1")
        .output()
        .unwrap_or_else(|error| panic!("the copied rollback helper should run: {error}"));
    assert!(
        output.status.success(),
        "the helper should report the failed replacement after restoring itself: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(helper.is_file(), "the original helper path should be restored");

    let rerun = Command::new(&helper)
        .args(["--ignored", "--exact", "integration::self_update_runtime::replaces_running_helper"])
        .output()
        .unwrap_or_else(|error| panic!("the restored helper should still run: {error}"));
    assert!(
        rerun.status.success(),
        "the restored helper should remain runnable: {}",
        String::from_utf8_lossy(&rerun.stderr)
    );
}

#[test]
#[ignore = "subprocess entry point; invoked by the replacement tests"]
fn replaces_running_helper() {
    let Some(replacement) = env::var_os(REPLACEMENT_ENV) else {
        return;
    };

    let result = replace_running_executable(Path::new(&replacement));
    if env::var_os(EXPECT_FAILURE_ENV).is_some() {
        assert!(result.is_err(), "the missing replacement should fail");
    } else {
        result.unwrap_or_else(|error| {
            panic!("the running helper should schedule replacement: {error}")
        });
    }
}
