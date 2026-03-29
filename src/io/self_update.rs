use std::{
    cmp::Ordering,
    fs::{self, File},
    io::{self, IsTerminal, Read},
    path::Path,
    time::Duration,
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
    let install_decision = decide_install(&local_version, &github_version, binaries_are_identical);
    reporter.decision(
        matches!(install_decision, InstallDecision::Install),
        &build_install_decision_message(install_decision),
    );

    if !matches!(install_decision, InstallDecision::Install) {
        match install_decision {
            InstallDecision::LocalNewer => {
                reporter.info("Local mdv is newer than GitHub main, so no downgrade was applied.");
            }
            InstallDecision::AlreadyLatest => {
                reporter.info(
                    "GitHub main differs at the binary level, but the current version is already the latest.",
                );
            }
            InstallDecision::BinaryIdentical | InstallDecision::Install => {}
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
    let response = http_agent()
        .get(url)
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
    let response = http_agent()
        .get(url)
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

fn http_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .build()
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

        return parse_manifest_version_value(value);
    }

    bail!("package.version was not found")
}

fn parse_manifest_version_value(raw_value: &str) -> Result<String> {
    let mut quote = None;
    let mut sanitized = String::new();

    for character in raw_value.trim().chars() {
        match quote {
            Some(active_quote) if character == active_quote => {
                quote = None;
                sanitized.push(character);
            }
            Some(_) => sanitized.push(character),
            None if matches!(character, '"' | '\'') => {
                quote = Some(character);
                sanitized.push(character);
            }
            None if character == '#' => break,
            None => sanitized.push(character),
        }
    }

    let sanitized = sanitized.trim();
    if sanitized.len() >= 2 {
        let first = sanitized.chars().next();
        let last = sanitized.chars().last();
        if matches!(first.zip(last), Some(('\'', '\'')) | Some(('"', '"'))) {
            return Ok(sanitized[1..sanitized.len() - 1].to_string());
        }
    }
    if sanitized.is_empty() {
        bail!("package.version was empty")
    }
    Ok(sanitized.to_string())
}

fn build_install_decision_message(decision: InstallDecision) -> String {
    if matches!(decision, InstallDecision::Install) {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstallDecision {
    Install,
    BinaryIdentical,
    AlreadyLatest,
    LocalNewer,
}

fn decide_install(
    local_version: &str,
    github_version: &str,
    binaries_are_identical: bool,
) -> InstallDecision {
    if binaries_are_identical {
        return InstallDecision::BinaryIdentical;
    }

    match compare_versions(local_version, github_version) {
        Some(Ordering::Less) => InstallDecision::Install,
        Some(Ordering::Equal) => InstallDecision::AlreadyLatest,
        Some(Ordering::Greater) => InstallDecision::LocalNewer,
        None => InstallDecision::Install,
    }
}

fn compare_versions(left: &str, right: &str) -> Option<Ordering> {
    let left_version = parse_version_parts(left)?;
    let right_version = parse_version_parts(right)?;
    let len = left_version.core.len().max(right_version.core.len());

    for index in 0..len {
        let left_part = *left_version.core.get(index).unwrap_or(&0);
        let right_part = *right_version.core.get(index).unwrap_or(&0);
        match left_part.cmp(&right_part) {
            Ordering::Equal => continue,
            ordering => return Some(ordering),
        }
    }

    compare_prerelease_identifiers(&left_version.prerelease, &right_version.prerelease)
}

fn compare_prerelease_identifiers(
    left: &[PrereleaseIdentifier],
    right: &[PrereleaseIdentifier],
) -> Option<Ordering> {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => Some(Ordering::Equal),
        (true, false) => Some(Ordering::Greater),
        (false, true) => Some(Ordering::Less),
        (false, false) => {
            let len = left.len().max(right.len());
            for index in 0..len {
                match (left.get(index), right.get(index)) {
                    (Some(left_identifier), Some(right_identifier)) => {
                        match compare_prerelease_identifier(left_identifier, right_identifier) {
                            Ordering::Equal => continue,
                            ordering => return Some(ordering),
                        }
                    }
                    (Some(_), None) => return Some(Ordering::Greater),
                    (None, Some(_)) => return Some(Ordering::Less),
                    (None, None) => break,
                }
            }
            Some(Ordering::Equal)
        }
    }
}

fn compare_prerelease_identifier(
    left: &PrereleaseIdentifier,
    right: &PrereleaseIdentifier,
) -> Ordering {
    match (left, right) {
        (PrereleaseIdentifier::Numeric(left), PrereleaseIdentifier::Numeric(right)) => {
            left.cmp(right)
        }
        (PrereleaseIdentifier::Numeric(_), PrereleaseIdentifier::Text(_)) => Ordering::Less,
        (PrereleaseIdentifier::Text(_), PrereleaseIdentifier::Numeric(_)) => Ordering::Greater,
        (PrereleaseIdentifier::Text(left), PrereleaseIdentifier::Text(right)) => left.cmp(right),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParsedVersion {
    core: Vec<u64>,
    prerelease: Vec<PrereleaseIdentifier>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PrereleaseIdentifier {
    Numeric(u64),
    Text(String),
}

fn parse_version_parts(version: &str) -> Option<ParsedVersion> {
    let trimmed = version.trim_start_matches('v');
    let without_build = trimmed.split_once('+').map_or(trimmed, |(core, _)| core);
    let (core, prerelease) = without_build
        .split_once('-')
        .map_or((without_build, None), |(core, prerelease)| (core, Some(prerelease)));
    if core.is_empty() {
        return None;
    }

    let core = core.split('.').map(|part| part.parse::<u64>().ok()).collect::<Option<Vec<_>>>()?;
    let prerelease = match prerelease {
        Some(prerelease) => parse_prerelease_identifiers(prerelease)?,
        None => Vec::new(),
    };

    Some(ParsedVersion { core, prerelease })
}

fn parse_prerelease_identifiers(prerelease: &str) -> Option<Vec<PrereleaseIdentifier>> {
    prerelease
        .split('.')
        .map(|identifier| {
            if identifier.is_empty() {
                return None;
            }
            Some(identifier.parse::<u64>().map_or_else(
                |_| PrereleaseIdentifier::Text(identifier.to_string()),
                PrereleaseIdentifier::Numeric,
            ))
        })
        .collect()
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
    use std::cmp::Ordering;

    use super::{
        InstallDecision, build_already_current_message, build_install_decision_message,
        build_success_message, compare_versions, decide_install,
        parse_package_version_from_manifest,
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
    fn parses_single_quoted_manifest_versions_with_inline_comments() {
        let manifest = r#"
[package]
name = "mdv"
version = '0.4.2-beta.1' # main build
"#;

        let version = parse_package_version_from_manifest(manifest)
            .unwrap_or_else(|error| panic!("single-quoted package version should parse: {error}"));
        assert_eq!(version, "0.4.2-beta.1");
    }

    #[test]
    fn install_decision_message_reports_yes_when_install_is_needed() {
        assert_eq!(
            build_install_decision_message(InstallDecision::Install),
            "Latest install required: yes"
        );
    }

    #[test]
    fn install_decision_message_reports_no_when_already_current() {
        assert_eq!(
            build_install_decision_message(InstallDecision::AlreadyLatest),
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
        assert_eq!(decide_install("0.3.0", "0.2.9", false), InstallDecision::LocalNewer);
    }

    #[test]
    fn requests_install_when_github_main_is_newer_than_local() {
        assert_eq!(decide_install("0.2.9", "0.3.0", false), InstallDecision::Install);
    }

    #[test]
    fn compares_prereleases_as_older_than_stable_releases() {
        assert_eq!(compare_versions("1.0.0-beta.1", "1.0.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.0.0", "1.0.0-beta.1"), Some(Ordering::Greater));
    }

    #[test]
    fn compares_prerelease_identifiers_using_semver_ordering() {
        assert_eq!(compare_versions("1.0.0-beta.2", "1.0.0-beta.11"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.0.0-beta.1", "1.0.0-beta.alpha"), Some(Ordering::Less));
    }
}
