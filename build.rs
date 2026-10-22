use std::{env, error::Error, path::PathBuf, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/io/webkit_snapshot/snapshot.swift");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
    if env::var("CARGO_CFG_TARGET_OS")? != "macos" {
        return Ok(());
    }

    let architecture = match env::var("CARGO_CFG_TARGET_ARCH")?.as_str() {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        other => return Err(format!("unsupported macOS snapshot architecture: {other}").into()),
    };
    let deployment_target = env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "11.0".into());
    let target = format!("{architecture}-apple-macosx{deployment_target}");
    let output_path =
        PathBuf::from(env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?).join("mdv-snapshot-helper");
    let output = Command::new("xcrun")
        .args(["--sdk", "macosx", "swiftc", "-O", "-target", &target])
        .arg("src/io/webkit_snapshot/snapshot.swift")
        .arg("-o")
        .arg(output_path)
        .output()
        .map_err(|error| {
            format!("building the WebKit helper requires the Swift toolchain: {error}")
        })?;
    if !output.status.success() {
        return Err(format!(
            "failed to compile the WebKit helper: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(())
}
