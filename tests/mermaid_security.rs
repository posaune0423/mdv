use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use mdv::{cli::Theme, io::mermaid_cli::MermaidCliRenderer};
use tempfile::TempDir;

const NO_NPX_PROBE: &str = "MDV_MERMAID_NO_NPX_PROBE";
const MMD_C_PROBE: &str = "MDV_MERMAID_MMDC_PROBE";

#[test]
fn does_not_execute_npx_from_path_when_mmdc_is_unavailable() {
    if let Some(marker) = env::var_os(NO_NPX_PROBE) {
        let renderer = MermaidCliRenderer::from_env();
        let _ = renderer.render_svg_sized("graph TD\n    A --> NoNpx\n", None, None, Theme::Light);

        assert!(!Path::new(&marker).exists(), "the renderer must not execute the npx fallback");
        return;
    }

    let directory = temporary_directory();
    let marker = directory.path().join("npx-invoked");
    write_fake_npx(directory.path(), &marker);

    let output = run_probe(
        "does_not_execute_npx_from_path_when_mmdc_is_unavailable",
        directory.path(),
        NO_NPX_PROBE,
        &marker,
    );

    assert!(
        output.status.success(),
        "isolated probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn uses_an_installed_mmdc_without_executing_npx() {
    if let Some(marker) = env::var_os(MMD_C_PROBE) {
        let renderer = MermaidCliRenderer::from_env();
        let svg = renderer
            .render_svg_sized("graph TD\n    A --> InstalledMmdc\n", None, None, Theme::Light)
            .unwrap_or_else(|error| panic!("installed mmdc should render: {error}"));

        assert_eq!(svg, "<svg></svg>");
        assert!(!Path::new(&marker).exists(), "npx must not run when mmdc is installed");
        return;
    }

    let directory = temporary_directory();
    let marker = directory.path().join("npx-invoked");
    write_fake_mmdc(directory.path());
    write_fake_npx(directory.path(), &marker);

    let output = run_probe(
        "uses_an_installed_mmdc_without_executing_npx",
        directory.path(),
        MMD_C_PROBE,
        &marker,
    );

    assert!(
        output.status.success(),
        "isolated probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn removes_the_render_workspace_when_the_renderer_fails() {
    let directory = temporary_directory();
    let workspace_record = directory.path().join("workspace-path");
    let command = write_workspace_recording_failure(directory.path(), &workspace_record);
    let renderer = MermaidCliRenderer::new(command);

    assert!(renderer.render_png("graph TD\n    A --> B\n", Theme::Light).is_err());

    let workspace = fs::read_to_string(&workspace_record)
        .unwrap_or_else(|error| panic!("workspace path should be recorded: {error}"));
    assert!(
        !Path::new(workspace.trim()).exists(),
        "the render workspace should be removed after a renderer failure"
    );
}

#[cfg(not(windows))]
#[test]
fn removes_a_private_render_workspace_after_a_successful_render() {
    let directory = temporary_directory();
    let workspace_record = directory.path().join("workspace-path");
    let command = write_workspace_recording_success(directory.path(), &workspace_record);
    let renderer = MermaidCliRenderer::new(command);

    let svg = renderer
        .render_svg_sized("graph TD\n    A --> B\n", None, None, Theme::Light)
        .unwrap_or_else(|error| panic!("renderer should succeed: {error}"));

    assert_eq!(svg, "<svg></svg>");
    let workspace = fs::read_to_string(&workspace_record)
        .unwrap_or_else(|error| panic!("workspace path should be recorded: {error}"));
    assert!(
        !Path::new(workspace.trim()).exists(),
        "the render workspace should be removed after a successful render"
    );
}

fn run_probe(
    test_name: &str,
    path: &Path,
    marker_name: &str,
    marker: &Path,
) -> std::process::Output {
    let mut command = Command::new(
        env::current_exe().unwrap_or_else(|error| panic!("test executable should exist: {error}")),
    );
    command
        .arg("--exact")
        .arg(test_name)
        .env_clear()
        .env("PATH", path)
        .env("XDG_CACHE_HOME", path.join("cache"))
        .env(marker_name, marker);
    retain_windows_command_environment(&mut command);
    command.output().unwrap_or_else(|error| panic!("isolated probe should run: {error}"))
}

#[cfg(windows)]
fn retain_windows_command_environment(command: &mut Command) {
    for variable in ["SystemRoot", "COMSPEC", "TEMP", "TMP"] {
        if let Some(value) = env::var_os(variable) {
            command.env(variable, value);
        }
    }
    command.env("PATHEXT", ".COM;.EXE;.BAT;.CMD");
}

#[cfg(not(windows))]
fn retain_windows_command_environment(_command: &mut Command) {}

fn temporary_directory() -> TempDir {
    TempDir::new().unwrap_or_else(|error| panic!("temporary directory should be created: {error}"))
}

#[cfg(not(windows))]
fn write_fake_npx(directory: &Path, marker: &Path) {
    let command = directory.join("npx");
    fs::write(&command, format!("#!/bin/sh\nprintf invoked > '{}'\nexit 1\n", marker.display()))
        .unwrap_or_else(|error| panic!("fake npx should be written: {error}"));
    make_executable(&command);
}

#[cfg(windows)]
fn write_fake_npx(directory: &Path, marker: &Path) {
    let command = directory.join("npx.cmd");
    fs::write(
        &command,
        format!("@echo off\r\necho invoked>\"{}\"\r\nexit /b 1\r\n", marker.display()),
    )
    .unwrap_or_else(|error| panic!("fake npx should be written: {error}"));
}

#[cfg(not(windows))]
fn write_fake_mmdc(directory: &Path) {
    let command = directory.join("mmdc");
    fs::write(
        &command,
        "#!/bin/sh\nset -eu\nout=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then out=\"$2\"; fi\n  shift\ndone\nprintf '<svg></svg>' > \"$out\"\n",
    )
    .unwrap_or_else(|error| panic!("fake mmdc should be written: {error}"));
    make_executable(&command);
}

#[cfg(windows)]
fn write_fake_mmdc(directory: &Path) {
    let command = directory.join("mmdc.cmd");
    fs::write(
        &command,
        "@echo off\r\nsetlocal EnableExtensions DisableDelayedExpansion\r\nset \"out=\"\r\n:arguments\r\nif \"%~1\"==\"\" goto done\r\nif \"%~1\"==\"-o\" set \"out=%~2\"\r\nshift\r\ngoto arguments\r\n:done\r\n<nul set /p \"=<svg></svg>\">\"%out%\"\r\n",
    )
    .unwrap_or_else(|error| panic!("fake mmdc should be written: {error}"));
}

#[cfg(not(windows))]
fn write_workspace_recording_failure(directory: &Path, record: &Path) -> PathBuf {
    let command = directory.join("record-workspace-failure");
    fs::write(
        &command,
        format!(
            "#!/bin/sh\nset -eu\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-i\" ]; then dirname \"$2\" > '{}'; fi\n  shift\ndone\nexit 7\n",
            record.display()
        ),
    )
    .unwrap_or_else(|error| panic!("failing renderer should be written: {error}"));
    make_executable(&command);
    command
}

#[cfg(not(windows))]
fn write_workspace_recording_success(directory: &Path, record: &Path) -> PathBuf {
    let command = directory.join("record-workspace-success");
    fs::write(
        &command,
        format!(
            "#!/bin/sh\nset -eu\nout=\"\"\nworkspace=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-i\" ]; then workspace=$(dirname \"$2\"); fi\n  if [ \"$1\" = \"-o\" ]; then out=\"$2\"; fi\n  shift\ndone\ncase \"$(uname -s)\" in\n  Darwin) mode=$(stat -f %Lp \"$workspace\") ;;\n  *) mode=$(stat -c %a \"$workspace\") ;;\nesac\n[ \"$mode\" = 700 ]\nprintf '%s' \"$workspace\" > '{}'\nprintf '<svg></svg>' > \"$out\"\n",
            record.display()
        ),
    )
    .unwrap_or_else(|error| panic!("successful renderer should be written: {error}"));
    make_executable(&command);
    command
}

#[cfg(windows)]
fn write_workspace_recording_failure(directory: &Path, record: &Path) -> PathBuf {
    let command = directory.join("record-workspace-failure.cmd");
    fs::write(
        &command,
        format!(
            "@echo off\r\nsetlocal EnableExtensions DisableDelayedExpansion\r\n:arguments\r\nif \"%~1\"==\"\" goto done\r\nif \"%~1\"==\"-i\" for %%I in (\"%~2\\..\") do echo %%~fI>\"{}\"\r\nshift\r\ngoto arguments\r\n:done\r\nexit /b 7\r\n",
            record.display()
        ),
    )
    .unwrap_or_else(|error| panic!("failing renderer should be written: {error}"));
    command
}

#[cfg(not(windows))]
fn make_executable(command: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(command)
        .unwrap_or_else(|error| panic!("fake command metadata should be read: {error}"))
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(command, permissions)
        .unwrap_or_else(|error| panic!("fake command should be executable: {error}"));
}
