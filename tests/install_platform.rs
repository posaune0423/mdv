#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, process::Command};

fn run_installer(os: &str, arch: &str, translated: bool, supported: bool) {
    let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let tools = directory.path().join("tools");
    let install = directory.path().join("install");
    fs::create_dir(&tools).unwrap_or_else(|error| panic!("tools: {error}"));
    fs::create_dir(&install).unwrap_or_else(|error| panic!("install: {error}"));
    let binary = install.join("mdv");
    fs::write(&binary, "existing-install").unwrap_or_else(|error| panic!("binary: {error}"));
    let marker = directory.path().join("download-attempted");
    let uname = tools.join("uname");
    fs::write(
        &uname,
        format!(
            "#!/bin/sh\ncase \"$1\" in -s) echo '{os}' ;; -m) echo '{arch}' ;; *) exit 1 ;; esac\n"
        ),
    )
    .unwrap_or_else(|error| panic!("uname: {error}"));
    let curl = tools.join("curl");
    fs::write(&curl, "#!/bin/sh\nprintf attempted > \"$MDV_DOWNLOAD_MARKER\"\nexit 23\n")
        .unwrap_or_else(|error| panic!("curl: {error}"));
    let sysctl = tools.join("sysctl");
    fs::write(&sysctl, format!("#!/bin/sh\necho {}\n", u8::from(translated)))
        .unwrap_or_else(|error| panic!("sysctl: {error}"));
    for path in [&uname, &curl, &sysctl] {
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|error| panic!("permissions: {error}"));
    }
    let output = Command::new("/bin/sh")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/install.sh"))
        .env("PATH", format!("{}:/usr/bin:/bin", tools.display()))
        .env("MDV_INSTALL_DIR", &install)
        .env("MDV_DOWNLOAD_MARKER", &marker)
        .output()
        .unwrap_or_else(|error| panic!("installer: {error}"));
    assert!(!output.status.success());
    assert_eq!(fs::read_to_string(binary).ok().as_deref(), Some("existing-install"));
    assert_eq!(marker.exists(), supported, "unsupported hosts must fail before downloading");
    if !supported {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("unsupported platform"), "{stderr}");
        assert!(stderr.contains(os) && stderr.contains(arch), "{stderr}");
    }
}

#[test]
fn rejects_unsupported_hosts_before_download_or_install() {
    for (os, arch) in
        [("Linux", "x86_64"), ("Linux", "aarch64"), ("Darwin", "x86_64"), ("FreeBSD", "amd64")]
    {
        run_installer(os, arch, false, false);
    }
}

#[test]
fn macos_arm64_reaches_download_without_replacing_install_on_failure() {
    run_installer("Darwin", "arm64", false, true);
}

#[test]
fn apple_silicon_under_rosetta_reaches_download() {
    run_installer("Darwin", "x86_64", true, true);
}
