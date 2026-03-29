use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use tracing::info_span;

use crate::cli::Theme;

static WORKSPACE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct MermaidCliRenderer {
    command: PathBuf,
    base_args: Vec<OsString>,
    cache_dir: PathBuf,
}

impl MermaidCliRenderer {
    #[must_use]
    pub fn from_env() -> Self {
        if let Some(command) = std::env::var_os("MDV_MERMAID_CMD") {
            return Self {
                command: PathBuf::from(command),
                base_args: Vec::new(),
                cache_dir: default_cache_root().join("mermaid"),
            };
        }

        if command_exists("mmdc") {
            return Self {
                command: PathBuf::from("mmdc"),
                base_args: Vec::new(),
                cache_dir: default_cache_root().join("mermaid"),
            };
        }

        if command_exists("npx") {
            return Self {
                command: PathBuf::from("npx"),
                base_args: vec![OsString::from("-y"), OsString::from("@mermaid-js/mermaid-cli")],
                cache_dir: default_cache_root().join("mermaid"),
            };
        }

        Self {
            command: PathBuf::from("mmdc"),
            base_args: Vec::new(),
            cache_dir: default_cache_root().join("mermaid"),
        }
    }

    #[must_use]
    pub fn new(command: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            base_args: Vec::new(),
            cache_dir: default_cache_root().join("mermaid"),
        }
    }

    #[must_use]
    pub fn with_cache_dir(command: impl Into<PathBuf>, cache_dir: PathBuf) -> Self {
        Self { command: command.into(), base_args: Vec::new(), cache_dir }
    }

    pub fn render_png(&self, source: &str, theme: Theme) -> Result<Vec<u8>> {
        self.render_png_sized(source, None, None, theme)
    }

    pub fn render_svg_sized(
        &self,
        source: &str,
        width_px: Option<u32>,
        scale: Option<f32>,
        theme: Theme,
    ) -> Result<String> {
        let svg_bytes = self.render_bytes_sized(source, "svg", width_px, scale, theme)?;
        String::from_utf8(svg_bytes).context("mermaid svg output was not valid utf-8")
    }

    pub fn render_png_sized(
        &self,
        source: &str,
        width_px: Option<u32>,
        scale: Option<f32>,
        theme: Theme,
    ) -> Result<Vec<u8>> {
        self.render_bytes_sized(source, "png", width_px, scale, theme)
    }

    fn render_bytes_sized(
        &self,
        source: &str,
        extension: &str,
        width_px: Option<u32>,
        scale: Option<f32>,
        theme: Theme,
    ) -> Result<Vec<u8>> {
        let _span = info_span!(
            "mermaid.render",
            extension,
            width_px = width_px.unwrap_or_default(),
            scale = scale.unwrap_or_default(),
            theme = theme.as_str()
        )
        .entered();
        let cache_path = self.cache_path(source, extension, width_px, scale, theme);
        if let Ok(cached) = fs::read(&cache_path) {
            return Ok(cached);
        }

        let workspace = create_workspace()?;
        let input = workspace.join("diagram.mmd");
        let output = workspace.join(format!("diagram.{extension}"));

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
            .arg("-t")
            .arg(mermaid_theme_name(theme));

        if let Some(width_px) = width_px.filter(|width_px| *width_px > 0) {
            command.arg("-w").arg(width_px.to_string());
        }
        if let Some(scale) = scale.filter(|scale| scale.is_finite() && *scale > 0.0) {
            command.arg("-s").arg(scale.to_string());
        }

        command.stdout(Stdio::null()).stderr(Stdio::piped());

        let command_output = command
            .output()
            .with_context(|| format!("failed to execute {}", self.command.display()))?;

        if !command_output.status.success() {
            let stderr = String::from_utf8_lossy(&command_output.stderr).trim().to_string();
            if stderr.is_empty() {
                bail!("mermaid renderer exited with {}", command_output.status);
            }
            bail!("mermaid renderer exited with {}: {}", command_output.status, stderr);
        }

        let rendered = fs::read(&output)
            .with_context(|| format!("mermaid renderer did not produce {}", output.display()))?;
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&cache_path, &rendered);
        let _ = fs::remove_dir_all(workspace);
        Ok(rendered)
    }

    fn cache_path(
        &self,
        source: &str,
        extension: &str,
        width_px: Option<u32>,
        scale: Option<f32>,
        theme: Theme,
    ) -> PathBuf {
        let scale_key = scale
            .filter(|value| value.is_finite() && *value > 0.0)
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "default".to_string());
        let width_key = width_px.unwrap_or_default().to_string();
        let theme_key = mermaid_theme_name(theme);
        let base_args = self
            .base_args
            .iter()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\u{1f}");
        let key = stable_cache_key(&[
            self.command.to_string_lossy().as_ref(),
            base_args.as_str(),
            extension,
            width_key.as_str(),
            scale_key.as_str(),
            theme_key,
            source,
        ]);

        self.cache_dir.join(format!("{key}.{extension}"))
    }
}

fn mermaid_theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::Light | Theme::System => "default",
        Theme::Dark => "dark",
    }
}

fn create_workspace() -> Result<PathBuf> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let counter = WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mdv-mermaid-{timestamp}-{counter}"));
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

fn default_cache_root() -> PathBuf {
    if cfg!(target_os = "macos")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join("Library/Caches/mdv");
    }

    if let Some(root) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(root).join("mdv");
    }

    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache/mdv");
    }

    std::env::temp_dir().join("mdv-cache")
}

fn stable_cache_key(parts: &[&str]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= u64::from(0xff_u8);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
