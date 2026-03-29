use std::{fs, path::Path};

use serde_yaml::{Mapping, Value};

#[test]
fn install_script_downloads_the_tracked_main_binary_with_feedback() {
    let script = fs::read_to_string("scripts/install.sh").unwrap_or_else(|error| {
        panic!("install script should be readable from the repository root: {error}")
    });

    assert!(
        script.contains("raw.githubusercontent.com/${REPO}/main/bin/mdv"),
        "install script should download the tracked main binary"
    );
    assert!(script.contains("spinner"), "install script should provide loading feedback");
    assert!(script.contains("_____"), "install script should print the success ASCII banner");
}

#[test]
fn ci_workflow_refreshes_the_tracked_binary_on_main_pushes() {
    let workflow = fs::read_to_string(".github/workflows/ci.yml")
        .unwrap_or_else(|error| panic!("ci workflow should be readable: {error}"));
    let parsed = parse_yaml(&workflow);
    let root = parsed.as_mapping().unwrap_or_else(|| panic!("workflow root should be a mapping"));
    let jobs = mapping_field(root, "jobs");
    let triggers = mapping_field(root, "on");
    let push = mapping_field(triggers, "push");
    let branches = sequence_field(push, "branches");

    assert!(
        jobs.contains_key(Value::String("checks".to_string())),
        "ci workflow should keep the core checks job"
    );
    assert!(
        jobs.contains_key(Value::String("refresh_tracked_binary".to_string())),
        "ci workflow should refresh bin/mdv from CI on main pushes"
    );
    assert!(
        workflow.contains("github.actor != 'github-actions[bot]'"),
        "ci workflow should avoid infinite bot-triggered refresh loops"
    );
    assert!(
        workflow.contains("git add bin/mdv"),
        "ci workflow should commit refreshed bin/mdv back to main"
    );
    assert!(
        branches.iter().any(|branch| branch.as_str() == Some("main")),
        "ci workflow should run on pushes to main"
    );
    assert!(
        !jobs.contains_key(Value::String("release_assets".to_string())),
        "ci workflow should no longer define the old release asset job"
    );
}

#[test]
fn release_automation_files_are_removed() {
    assert!(
        !Path::new(".github/workflows/main-channel.yml").exists(),
        "main-channel workflow should be removed"
    );
    assert!(
        !Path::new(".github/workflows/release-assets.yml").exists(),
        "release-assets workflow should be removed"
    );
    assert!(
        !Path::new(".github/workflows/release.yml").exists(),
        "release workflow should be removed"
    );
}

#[test]
fn lefthook_config_uses_repo_quality_gates() {
    let config = fs::read_to_string("lefthook.yml")
        .unwrap_or_else(|error| panic!("lefthook config should be readable: {error}"));
    let parsed = parse_yaml(&config);
    let root = parsed.as_mapping().unwrap_or_else(|| panic!("lefthook root should be a mapping"));
    let pre_commit = mapping_field(root, "pre-commit");
    let pre_push = mapping_field(root, "pre-push");
    let pre_commit_commands = mapping_field(pre_commit, "commands");
    let pre_push_commands = mapping_field(pre_push, "commands");

    assert!(
        config.contains("cargo fmt --all -- --check"),
        "pre-commit should run rustfmt in check mode"
    );
    assert!(
        config.contains("cargo check --workspace --all-targets --all-features"),
        "pre-commit should run cargo check across the full workspace"
    );
    assert!(config.contains("make ci"), "pre-push should reuse the repo CI entrypoint");
    assert!(
        pre_commit_commands.contains_key(Value::String("fmt".to_string())),
        "pre-commit should define the fmt command"
    );
    assert!(
        pre_commit_commands.contains_key(Value::String("check".to_string())),
        "pre-commit should define the check command"
    );
    assert!(
        pre_push_commands.contains_key(Value::String("ci".to_string())),
        "pre-push should define the ci command"
    );
}

fn parse_yaml(source: &str) -> Value {
    serde_yaml::from_str::<Value>(source)
        .unwrap_or_else(|error| panic!("workflow should be valid YAML: {error}"))
}

fn mapping_field<'a>(mapping: &'a Mapping, key: &str) -> &'a Mapping {
    mapping
        .get(Value::String(key.to_string()))
        .and_then(Value::as_mapping)
        .unwrap_or_else(|| panic!("workflow field `{key}` should be a mapping"))
}

fn sequence_field<'a>(mapping: &'a Mapping, key: &str) -> &'a [Value] {
    mapping
        .get(Value::String(key.to_string()))
        .and_then(Value::as_sequence)
        .map(Vec::as_slice)
        .unwrap_or_else(|| panic!("workflow field `{key}` should be a sequence"))
}
