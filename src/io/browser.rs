use std::process::Command;

use anyhow::{Result, bail};

#[must_use]
pub fn browser_command_for(os: &str, url: &str) -> Option<(String, Vec<String>)> {
    match os {
        "macos" => Some(("open".to_string(), vec![url.to_string()])),
        "linux" => Some(("xdg-open".to_string(), vec![url.to_string()])),
        _ => None,
    }
}

pub fn open_url(url: &str) -> Result<()> {
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
