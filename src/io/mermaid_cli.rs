use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};

#[derive(Clone, Debug)]
pub struct MermaidCliRenderer {
    command: PathBuf,
    base_args: Vec<OsString>,
}

impl MermaidCliRenderer {
    #[must_use]
    pub fn from_env() -> Self {
        if let Some(command) = std::env::var_os("MDV_MERMAID_CMD") {
            return Self { command: PathBuf::from(command), base_args: Vec::new() };
        }

        if command_exists("mmdc") {
            return Self { command: PathBuf::from("mmdc"), base_args: Vec::new() };
        }

        if command_exists("npx") {
            return Self {
                command: PathBuf::from("npx"),
                base_args: vec![OsString::from("-y"), OsString::from("@mermaid-js/mermaid-cli")],
            };
        }

        Self { command: PathBuf::from("mmdc"), base_args: Vec::new() }
    }

    #[must_use]
    pub fn new(command: impl Into<PathBuf>) -> Self {
        Self { command: command.into(), base_args: Vec::new() }
    }

    pub fn render_png(&self, source: &str) -> Result<Vec<u8>> {
        let workspace = create_workspace()?;
        let input = workspace.join("diagram.mmd");
        let output = workspace.join("diagram.png");

        fs::write(&input, source)?;
        let mut command = Command::new(&self.command);
        command
            .args(&self.base_args)
            .arg("-i")
            .arg(&input)
            .arg("-o")
            .arg(&output)
            .arg("-b")
            .arg("transparent")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let status = command
            .status()
            .with_context(|| format!("failed to execute {}", self.command.display()))?;

        if !status.success() {
            bail!("mermaid renderer exited with {status}");
        }

        let png = fs::read(&output)
            .with_context(|| format!("mermaid renderer did not produce {}", output.display()))?;
        let _ = fs::remove_dir_all(workspace);
        Ok(png)
    }
}

fn create_workspace() -> Result<PathBuf> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = std::env::temp_dir().join(format!("mdv-mermaid-{timestamp}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[allow(dead_code)]
fn _exists(path: &Path) -> bool {
    path.exists()
}

fn command_exists(command: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|path| {
            let candidate = path.join(command);
            candidate.exists() || candidate.with_extension("exe").exists()
        })
    })
}
