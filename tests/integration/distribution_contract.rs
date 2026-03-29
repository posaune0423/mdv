use std::fs;

#[test]
fn install_script_downloads_the_main_channel_binary() {
    let script = fs::read_to_string("scripts/install.sh").unwrap_or_else(|error| {
        panic!("install script should be readable from the repository root: {error}")
    });

    assert!(
        script.contains("MDV_CHANNEL:-main"),
        "install script should default to the rolling main channel"
    );
    assert!(
        script.contains("/releases/download/${channel}/"),
        "install script should resolve binaries from the selected published channel"
    );
}

#[test]
fn ci_workflow_uses_release_assets_naming() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .unwrap_or_else(|error| panic!("ci workflow should be readable: {error}"));

    assert!(
        workflow.contains("  release_assets:\n"),
        "ci workflow should use a release_assets job name"
    );
    assert!(
        workflow.contains("make release-assets-check"),
        "ci workflow should verify the release-assets check path"
    );
}

#[test]
fn release_workflow_is_release_please_driven() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .unwrap_or_else(|error| panic!("release workflow should be readable: {error}"));

    assert!(
        workflow.contains("googleapis/release-please-action@v4"),
        "release workflow should use release-please for automated versioning"
    );
    assert!(
        workflow.contains("branches:\n      - main"),
        "release workflow should run from main pushes instead of manual tag pushes"
    );
    assert!(
        !workflow.contains("tags:\n      - \"v*\""),
        "release workflow should not rely on tag-push triggers anymore"
    );
}
