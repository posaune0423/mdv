use std::{
    cmp::Ordering,
    fs::{self, File},
    io::{self, IsTerminal, Read},
    path::Path,
};

use anyhow::{Context, Result, bail};

const REPOSITORY: &str = "posaune0423/mdv";
const MAIN_BINARY_URL: &str = "https://raw.githubusercontent.com/posaune0423/mdv/main/bin/mdv";
const MAIN_MANIFEST_URL: &str = "https://raw.githubusercontent.com/posaune0423/mdv/main/Cargo.toml";

#[must_use]
pub const fn main_binary_url() -> &'static str {
    MAIN_BINARY_URL
}

#[must_use]
pub const fn main_manifest_url() -> &'static str {
    MAIN_MANIFEST_URL
}

pub fn update_current_executable() -> Result<()> {
    let reporter = UpdateReporter::new();
    let current_exe =
        std::env::current_exe().context("failed to resolve the current mdv executable path")?;
    reporter.title("mdv updater");
    reporter
        .info(&format!("Checking the tracked main-branch binary for {}.", current_exe.display()));

    reporter.loading("Checking GitHub main state");
    let github_version = fetch_github_main_version()?;
    reporter.ok(&format!("GitHub main version: {}", format_version(&github_version)));

    let local_version = current_version().to_string();
    reporter.ok(&format!("Local mdv version: {}", format_version(&local_version)));

    let temp_dir = tempfile::tempdir().context("failed to create a temporary update directory")?;
    let latest_binary_path = temp_dir.path().join("mdv-main");

    reporter.loading("Downloading latest main/bin/mdv");
    download_main_binary(main_binary_url(), &latest_binary_path)?;
    reporter.ok("Downloaded latest main/bin/mdv");

    let binaries_are_identical = binaries_match(&current_exe, &latest_binary_path)?;
    let needs_install =
        should_install_latest(&local_version, &github_version, binaries_are_identical);
    reporter.decision(
        needs_install,
        &build_install_decision_message(&local_version, &github_version, needs_install),
    );

    if !needs_install {
        if !binaries_are_identical
            && compare_versions(&local_version, &github_version) == Some(Ordering::Greater)
        {
            reporter.info("Local mdv is newer than GitHub main, so no downgrade was applied.");
        } else if !binaries_are_identical {
            reporter.info("GitHub main differs at the binary level, but the current version is already the latest.");
        }
        reporter.ok(&build_already_current_message(&local_version));
        return Ok(());
    }

    reporter.loading("Installing latest mdv");
    install_replacement(&current_exe, &latest_binary_path)?;
    reporter.ok(&build_success_message(&github_version));

    if directory_is_on_path(current_exe.parent().unwrap_or_else(|| Path::new("."))) {
        reporter.ok(&format!("PATH continues to resolve mdv from {}", current_exe.display()));
    } else {
        reporter.info(&format!(
            "Updated the executable in place, but {} is not currently on PATH.",
            current_exe.parent().unwrap_or_else(|| Path::new(".")).display()
        ));
    }

    Ok(())
}

fn fetch_github_main_version() -> Result<String> {
    let manifest = download_text(main_manifest_url())?;
    parse_package_version_from_manifest(&manifest).with_context(|| {
        format!("failed to parse the package version from {}", main_manifest_url())
    })
}

fn download_text(url: &str) -> Result<String> {
    let response = ureq::get(url)
        .set("User-Agent", &format!("{REPOSITORY}/{}", current_version()))
        .call()
        .with_context(|| format!("failed to download mdv metadata from {url}"))?;
    let mut reader = response.into_reader();
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .with_context(|| format!("failed to read mdv metadata from {url}"))?;
    Ok(content)
}

fn download_main_binary(url: &str, destination: &Path) -> Result<()> {
    let response = ureq::get(url)
        .set("User-Agent", &format!("{REPOSITORY}/{}", current_version()))
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

fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn format_version(version: &str) -> String {
    format!("v{version}")
}

fn parse_package_version_from_manifest(manifest: &str) -> Result<String> {
    let mut in_package_section = false;

    for raw_line in manifest.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_package_section = line == "[package]";
            continue;
        }

        if !in_package_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim() != "version" {
            continue;
        }

        return Ok(value.trim().trim_matches('"').to_string());
    }

    bail!("package.version was not found")
}

fn build_install_decision_message(
    _local_version: &str,
    _github_version: &str,
    needs_install: bool,
) -> String {
    if needs_install {
        "Latest install required: yes".to_string()
    } else {
        "Latest install required: no".to_string()
    }
}

fn build_success_message(version: &str) -> String {
    format!("Successfully updated to {}", format_version(version))
}

fn build_already_current_message(version: &str) -> String {
    format!("Already on {}", format_version(version))
}

fn should_install_latest(
    local_version: &str,
    github_version: &str,
    binaries_are_identical: bool,
) -> bool {
    if binaries_are_identical {
        return false;
    }

    match compare_versions(local_version, github_version) {
        Some(Ordering::Less) => true,
        Some(Ordering::Equal | Ordering::Greater) => false,
        None => true,
    }
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let left_parts = parse_version_parts(left)?;
    let right_parts = parse_version_parts(right)?;
    let len = left_parts.len().max(right_parts.len());

    for index in 0..len {
        let left_part = *left_parts.get(index).unwrap_or(&0);
        let right_part = *right_parts.get(index).unwrap_or(&0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => continue,
            ordering => return Some(ordering),
        }
    }

    Some(Ordering::Equal)
}

fn parse_version_parts(version: &str) -> Option<Vec<u64>> {
    let trimmed = version.trim_start_matches('v');
    let core = trimmed.split_once('-').map_or(trimmed, |(core, _)| core);
    if core.is_empty() {
        return None;
    }

    core.split('.').map(|part| part.parse::<u64>().ok()).collect()
}

struct UpdateReporter {
    tty_effects: bool,
}

impl UpdateReporter {
    fn new() -> Self {
        Self {
            tty_effects: io::stdout().is_terminal()
                && std::env::var("TERM").ok().as_deref() != Some("dumb"),
        }
    }

    fn title(&self, message: &str) {
        if self.tty_effects {
            println!("\u{1b}[1m{message}\u{1b}[0m");
        } else {
            println!("{message}");
        }
    }

    fn info(&self, message: &str) {
        self.print("..", message);
    }

    fn loading(&self, message: &str) {
        self.print("..", message);
    }

    fn ok(&self, message: &str) {
        self.print("ok", message);
    }

    fn decision(&self, needs_install: bool, message: &str) {
        self.print(if needs_install { "yes" } else { "no" }, message);
    }

    fn print(&self, status: &str, message: &str) {
        if self.tty_effects {
            let color = match status {
                "ok" => "\u{1b}[32m",
                "yes" => "\u{1b}[36m",
                "no" => "\u{1b}[2m",
                _ => "\u{1b}[36m",
            };
            println!("{color}[{status}]\u{1b}[0m {message}");
        } else {
            println!("[{status}] {message}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_already_current_message, build_install_decision_message, build_success_message,
        parse_package_version_from_manifest, should_install_latest,
    };

    #[test]
    fn parses_the_package_version_from_the_github_manifest() {
        let manifest = r#"
[package]
name = "mdv"
version = "0.4.2"
edition = "2024"

[dependencies]
anyhow = "1"
"#;

        let version = parse_package_version_from_manifest(manifest)
            .unwrap_or_else(|error| panic!("package version should parse: {error}"));
        assert_eq!(version, "0.4.2");
    }

    #[test]
    fn install_decision_message_reports_yes_when_install_is_needed() {
        assert_eq!(
            build_install_decision_message("0.1.0", "0.2.0", true),
            "Latest install required: yes"
        );
    }

    #[test]
    fn install_decision_message_reports_no_when_already_current() {
        assert_eq!(
            build_install_decision_message("0.2.0", "0.2.0", false),
            "Latest install required: no"
        );
    }

    #[test]
    fn success_message_includes_the_target_version() {
        assert_eq!(build_success_message("0.4.2"), "Successfully updated to v0.4.2");
    }

    #[test]
    fn already_current_message_uses_the_local_version() {
        assert_eq!(build_already_current_message("0.4.2"), "Already on v0.4.2");
    }

    #[test]
    fn skips_install_when_local_version_is_newer_than_github_main() {
        assert!(!should_install_latest("0.3.0", "0.2.9", false));
    }

    #[test]
    fn requests_install_when_github_main_is_newer_than_local() {
        assert!(should_install_latest("0.2.9", "0.3.0", false));
    }
}
