use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use flate2::read::GzDecoder;
use tar::Archive;

const REPOSITORY: &str = "posaune0423/mdv";

#[must_use]
pub fn release_asset_name(os: &str, arch: &str) -> Option<String> {
    let target = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => return None,
    };

    Some(format!("mdv-{target}.tar.gz"))
}

pub fn update_current_executable() -> Result<()> {
    let asset =
        release_asset_name(std::env::consts::OS, std::env::consts::ARCH).ok_or_else(|| {
            anyhow::anyhow!(
                "self-update is only supported on Linux x86_64/aarch64 and macOS x86_64/aarch64"
            )
        })?;
    let url = format!("https://github.com/{REPOSITORY}/releases/latest/download/{asset}");
    let current_exe =
        std::env::current_exe().context("failed to resolve the current mdv executable path")?;
    let temp_dir = tempfile::tempdir().context("failed to create a temporary update directory")?;
    let archive_path = temp_dir.path().join(&asset);

    download_release_archive(&url, &archive_path)?;
    let extracted = extract_archive_binary(&archive_path, temp_dir.path())?;
    install_replacement(&current_exe, &extracted)?;

    println!("Updated mdv at {}", current_exe.display());
    if directory_is_on_path(current_exe.parent().unwrap_or_else(|| Path::new("."))) {
        println!("PATH continues to resolve mdv from {}", current_exe.display());
    } else {
        println!(
            "Updated the executable in place, but {} is not currently on PATH.",
            current_exe.parent().unwrap_or_else(|| Path::new(".")).display()
        );
    }

    Ok(())
}

fn download_release_archive(url: &str, destination: &Path) -> Result<()> {
    let response = ureq::get(url)
        .set("User-Agent", &format!("mdv/{}", env!("CARGO_PKG_VERSION")))
        .call()
        .with_context(|| format!("failed to download the latest GitHub Release from {url}"))?;
    let mut reader = response.into_reader();
    let mut file = File::create(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    io::copy(&mut reader, &mut file)
        .with_context(|| format!("failed to write {}", destination.display()))?;
    Ok(())
}

fn extract_archive_binary(archive_path: &Path, temp_dir: &Path) -> Result<PathBuf> {
    let archive_file = File::open(archive_path)
        .with_context(|| format!("failed to open {}", archive_path.display()))?;
    let decoder = GzDecoder::new(archive_file);
    let mut archive = Archive::new(decoder);
    let extracted_path = temp_dir.join("mdv-extracted");

    for entry in archive.entries().context("failed to read release archive entries")? {
        let mut entry = entry.context("failed to read a release archive entry")?;
        if entry.path().context("failed to inspect a release archive entry path")?.as_ref()
            == Path::new("mdv")
        {
            entry
                .unpack(&extracted_path)
                .with_context(|| format!("failed to unpack {}", extracted_path.display()))?;
            return Ok(extracted_path);
        }
    }

    bail!("release archive did not contain a top-level mdv binary")
}

fn install_replacement(current_exe: &Path, extracted: &Path) -> Result<()> {
    let install_dir = current_exe
        .parent()
        .with_context(|| format!("{} has no parent directory", current_exe.display()))?;
    let staged = install_dir.join(format!(".mdv-update-{}", std::process::id()));

    if staged.exists() {
        fs::remove_file(&staged)
            .with_context(|| format!("failed to remove stale {}", staged.display()))?;
    }

    fs::copy(extracted, &staged)
        .with_context(|| format!("failed to stage {}", staged.display()))?;
    let permissions = fs::metadata(extracted)
        .with_context(|| format!("failed to read {}", extracted.display()))?
        .permissions();
    fs::set_permissions(&staged, permissions)
        .with_context(|| format!("failed to set permissions on {}", staged.display()))?;
    fs::rename(&staged, current_exe)
        .with_context(|| format!("failed to replace {}", current_exe.display()))?;

    Ok(())
}

fn directory_is_on_path(dir: &Path) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|path| path == dir))
}
