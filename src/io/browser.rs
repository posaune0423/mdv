#[cfg(not(windows))]
use std::process::Command;

#[cfg(windows)]
use anyhow::Context;
use anyhow::{Result, bail};
#[cfg(any(windows, test))]
use url::Url;

#[must_use]
pub fn browser_command_for(os: &str, url: &str) -> Option<(String, Vec<String>)> {
    match os {
        "macos" => Some(("open".to_string(), vec![url.to_string()])),
        "linux" => Some(("xdg-open".to_string(), vec![url.to_string()])),
        _ => None,
    }
}

pub fn open_url(url: &str) -> Result<()> {
    #[cfg(windows)]
    {
        if !is_safe_windows_browser_url(url) {
            bail!("refusing to open a non-web link with the Windows shell");
        }
        open::that_detached(url).context("failed to open link with the Windows shell")?;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let os = std::env::consts::OS;
        let Some((program, args)) = browser_command_for(os, url) else {
            bail!("opening links is unsupported on {os}");
        };

        let status = Command::new(program).args(args).status()?;
        if !status.success() {
            bail!("browser command failed");
        }

        Ok(())
    }
}

#[cfg(any(windows, test))]
fn is_safe_windows_browser_url(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https" | "mailto"))
}

#[cfg(test)]
mod tests {
    use super::is_safe_windows_browser_url;

    #[test]
    fn windows_browser_url_validation_allows_only_web_and_mailto_targets() {
        assert!(is_safe_windows_browser_url("https://example.com/path?q=1"));
        assert!(is_safe_windows_browser_url("mailto:reader@example.com"));

        let local_file = tempfile::NamedTempFile::new()
            .unwrap_or_else(|error| panic!("local link fixture should be created: {error}"));
        let local_file_url = url::Url::from_file_path(local_file.path())
            .unwrap_or_else(|_| panic!("local link fixture should produce a file URL"));
        assert!(!is_safe_windows_browser_url(local_file_url.as_str()));
        assert!(!is_safe_windows_browser_url("./relative-link.md"));
        assert!(!is_safe_windows_browser_url("shell:AppsFolder\\cmd.exe"));
        assert!(!is_safe_windows_browser_url("ftp://example.com/archive"));
    }
}
