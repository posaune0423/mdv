use std::{
    fs::{self, File},
    io,
    path::Path,
};

use anyhow::{Context, Result};

const REPOSITORY: &str = "posaune0423/mdv";
const MAIN_BINARY_URL: &str = "https://raw.githubusercontent.com/posaune0423/mdv/main/bin/mdv";

#[must_use]
pub const fn main_binary_url() -> &'static str {
    MAIN_BINARY_URL
}

pub fn update_current_executable() -> Result<()> {
    let current_exe =
        std::env::current_exe().context("failed to resolve the current mdv executable path")?;
    let temp_dir = tempfile::tempdir().context("failed to create a temporary update directory")?;
    let latest_binary_path = temp_dir.path().join("mdv-main");

    download_main_binary(main_binary_url(), &latest_binary_path)?;
    if binaries_match(&current_exe, &latest_binary_path)? {
        println!("mdv is already up to date at {}", current_exe.display());
        return Ok(());
    }

    install_replacement(&current_exe, &latest_binary_path)?;

    println!("Updated mdv from {} to {}", main_binary_url(), current_exe.display());
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

fn download_main_binary(url: &str, destination: &Path) -> Result<()> {
    let response = ureq::get(url)
        .set("User-Agent", &format!("{REPOSITORY}/{}", env!("CARGO_PKG_VERSION")))
        .call()
        .with_context(|| format!("failed to download mdv from {url}"))?;
    let mut reader = response.into_reader();
    let mut file = File::create(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    io::copy(&mut reader, &mut file)
        .with_context(|| format!("failed to write {}", destination.display()))?;
    Ok(())
}

fn binaries_match(current_exe: &Path, latest_binary: &Path) -> Result<bool> {
    let current = fs::read(current_exe)
        .with_context(|| format!("failed to read {}", current_exe.display()))?;
    let latest = fs::read(latest_binary)
        .with_context(|| format!("failed to read {}", latest_binary.display()))?;
    Ok(current == latest)
}

fn install_replacement(current_exe: &Path, replacement: &Path) -> Result<()> {
    let install_dir = current_exe
        .parent()
        .with_context(|| format!("{} has no parent directory", current_exe.display()))?;
    let staged = install_dir.join(format!(".mdv-update-{}", std::process::id()));

    if staged.exists() {
        fs::remove_file(&staged)
            .with_context(|| format!("failed to remove stale {}", staged.display()))?;
    }

    fs::copy(replacement, &staged)
        .with_context(|| format!("failed to stage {}", staged.display()))?;
    let permissions = fs::metadata(current_exe)
        .with_context(|| format!("failed to read {}", current_exe.display()))?
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
