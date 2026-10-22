use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use headless_chrome::{
    Browser,
    protocol::cdp::{Emulation, Page, Runtime},
};
use serde::Deserialize;
use tracing::info_span;
use url::Url;

use crate::io::webkit_snapshot::{
    SnapshotAssetDiagnostics, SnapshotDiagnostics, SnapshotResult, SnapshotTypographyDiagnostics,
};

const BROWSER_START_TIMEOUT: Duration = Duration::from_secs(10);
const CDP_TIMEOUT: Duration = Duration::from_secs(5);
const RENDER_TIMEOUT: Duration = Duration::from_secs(25);
const VIEWPORT_HEIGHT_PX: u32 = 900;
const DEVICE_SCALE_FACTOR: f64 = 2.0;
const DEVICE_SCALE_FACTOR_PX: u64 = 2;
const MAX_RASTER_DIMENSION_PX: u64 = 32_000;
const MAX_RASTER_PIXELS: u64 = 128_000_000;
const MAX_READINESS_ATTEMPTS: u32 = 40;
const READINESS_INTERVAL: Duration = Duration::from_millis(25);

const READINESS_PROBE: &str = r#"
(() => {
  const height = Math.max(
    document.documentElement.scrollHeight || 0,
    document.body.scrollHeight || 0,
    document.documentElement.offsetHeight || 0,
    document.body.offsetHeight || 0
  );
  const fontsReady = !document.fonts || document.fonts.status === "loaded";
  const proseFontReady = !document.fonts || document.fonts.check('16px "Mona Sans VF"');
  const firstHeading = document.querySelector("h1, h2, h3, h4, h5, h6");
  const firstStrong = document.querySelector("strong, b");
  const isRemoteAsset = (value) => /^https?:\/\//i.test(value || "");
  const typographySelectors = [
    ["h1", "h1"], ["h2", "h2"], ["h3", "h3"], ["h4", "h4"],
    ["h5", "h5"], ["h6", "h6"], ["strong", "strong, b"],
    ["em", "em, i"], ["code", "p code, li code, blockquote code, td code, th code"]
  ];
  const typography = typographySelectors.map(([role, selector]) => {
    const element = document.querySelector(selector);
    const style = element ? getComputedStyle(element) : null;
    return {
      role, present: !!element,
      fontFamily: style ? style.fontFamily || "" : "",
      fontWeight: style ? style.fontWeight || "" : "",
      fontStyle: style ? style.fontStyle || "" : "",
      fontSize: style ? Number.parseFloat(style.fontSize || "0") || 0 : 0,
      lineHeight: style ? Number.parseFloat(style.lineHeight || "0") || 0 : 0
    };
  });
  const images = Array.from(document.images || []).map((image) => {
    const rect = image.getBoundingClientRect();
    const source = image.getAttribute("src") || "";
    const currentSrc = image.currentSrc || "";
    const resolvedSource = currentSrc || source;
    const remote = isRemoteAsset(resolvedSource);
    return {
      source, currentSrc, complete: !!image.complete,
      naturalWidth: image.naturalWidth || 0, naturalHeight: image.naturalHeight || 0,
      renderedWidth: rect.width || 0, renderedHeight: rect.height || 0,
      blocking: !remote, viewBox: "", contentLength: 0
    };
  });
  const mermaids = Array.from(document.querySelectorAll(".mdv-mermaid-diagram")).map((diagram) => {
    const rect = diagram.getBoundingClientRect();
    const viewBox = diagram.getAttribute("viewBox") || "";
    const viewBoxParts = viewBox.split(/[\s,]+/).map((part) => Number(part));
    return {
      source: viewBox ? `viewBox=${viewBox}` : "mermaid", currentSrc: "", complete: true,
      naturalWidth: Number.isFinite(viewBoxParts[2]) ? viewBoxParts[2] : (rect.width || 0),
      naturalHeight: Number.isFinite(viewBoxParts[3]) ? viewBoxParts[3] : (rect.height || 0),
      renderedWidth: rect.width || 0, renderedHeight: rect.height || 0,
      blocking: false, viewBox, contentLength: (diagram.innerHTML || "").length
    };
  });
  const imagesReady = images.every((image) => {
    if (!image.blocking) return true;
    if (!image.complete) return false;
    if (!image.currentSrc) return true;
    return image.naturalWidth > 0;
  });
  const remoteImagesReady = images.every((image) => !image.blocking || image.complete);
  const mermaidsReady = mermaids.every((diagram) => (
    diagram.renderedWidth > 0 && diagram.renderedHeight > 0
  ));
  return {
    height, fontsReady, proseFontReady, imagesReady, remoteImagesReady, mermaidsReady,
    headingFontWeight: firstHeading ? getComputedStyle(firstHeading).fontWeight || "" : "",
    strongFontWeight: firstStrong ? getComputedStyle(firstStrong).fontWeight || "" : "",
    typography, images, mermaids
  };
})()
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadinessProbe {
    #[serde(default)]
    height: f64,
    #[serde(default)]
    fonts_ready: bool,
    #[serde(default)]
    prose_font_ready: bool,
    #[serde(default)]
    images_ready: bool,
    #[serde(default)]
    remote_images_ready: bool,
    #[serde(default)]
    mermaids_ready: bool,
    #[serde(default)]
    heading_font_weight: String,
    #[serde(default)]
    strong_font_weight: String,
    #[serde(default)]
    typography: Vec<SnapshotTypographyDiagnostics>,
    #[serde(default)]
    images: Vec<ProbeAsset>,
    #[serde(default)]
    mermaids: Vec<ProbeAsset>,
}

impl ReadinessProbe {
    fn diagnostics(&self) -> SnapshotDiagnostics {
        SnapshotDiagnostics {
            fonts_ready: self.fonts_ready,
            prose_font_ready: self.prose_font_ready,
            images_ready: self.images_ready,
            mermaids_ready: self.mermaids_ready,
            heading_font_weight: self.heading_font_weight.clone(),
            strong_font_weight: self.strong_font_weight.clone(),
            typography: self.typography.clone(),
            images: self.images.iter().map(ProbeAsset::diagnostics).collect(),
            mermaids: self.mermaids.iter().map(ProbeAsset::diagnostics).collect(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeAsset {
    #[serde(default)]
    source: String,
    #[serde(default)]
    current_src: String,
    #[serde(default)]
    complete: bool,
    #[serde(default)]
    natural_width: f64,
    #[serde(default)]
    natural_height: f64,
    #[serde(default)]
    rendered_width: f64,
    #[serde(default)]
    rendered_height: f64,
    #[serde(default)]
    blocking: bool,
    #[serde(default)]
    view_box: String,
    #[serde(default)]
    content_length: usize,
}

impl ProbeAsset {
    fn diagnostics(&self) -> SnapshotAssetDiagnostics {
        SnapshotAssetDiagnostics {
            source: self.source.clone(),
            current_src: self.current_src.clone(),
            complete: self.complete,
            natural_width_px: self.natural_width,
            natural_height_px: self.natural_height,
            rendered_width_px: self.rendered_width,
            rendered_height_px: self.rendered_height,
            view_box: self.view_box.clone(),
            content_length: self.content_length,
        }
    }
}

struct ChromiumProcess {
    child: Arc<Mutex<Child>>,
    profile: tempfile::TempDir,
}

impl Drop for ChromiumProcess {
    fn drop(&mut self) {
        reap_browser_child(&self.child);
    }
}

struct ProcessWatchdog {
    cancel: mpsc::Sender<()>,
    join: Option<thread::JoinHandle<()>>,
}

impl ProcessWatchdog {
    fn arm(process: &ChromiumProcess, timeout: Duration) -> Self {
        let (cancel, receiver) = mpsc::channel();
        let child = Arc::clone(&process.child);
        let join = thread::spawn(move || {
            if receiver.recv_timeout(timeout).is_err() {
                reap_browser_child(&child);
            }
        });
        Self { cancel, join: Some(join) }
    }
}

impl Drop for ProcessWatchdog {
    fn drop(&mut self) {
        let _ = self.cancel.send(());
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn reap_browser_child(child: &Arc<Mutex<Child>>) {
    if let Ok(mut child) = child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub(crate) fn render_html_to_png(
    html: &str,
    base_dir: &Path,
    viewport_width_px: u32,
) -> Result<SnapshotResult> {
    ensure!(viewport_width_px > 0, "snapshot viewport width must be greater than zero");
    let _span = info_span!("chromium_snapshot.render_image", viewport_width_px).entered();
    let workspace = create_workspace(base_dir)?;
    let html_path = workspace.path().join("document.html");
    fs::write(&html_path, html).context("failed to write HTML snapshot input")?;
    let page_url = Url::from_file_path(&html_path)
        .map_err(|_| anyhow::anyhow!("failed to create a file URL for snapshot input"))?;

    render_file_url(page_url.as_str(), viewport_width_px)
}

fn render_file_url(page_url: &str, viewport_width_px: u32) -> Result<SnapshotResult> {
    let browser_path = browser_executable()?;
    let startup_deadline = Instant::now() + BROWSER_START_TIMEOUT;
    let mut process = launch_browser(&browser_path, viewport_width_px)?;
    let _watchdog = ProcessWatchdog::arm(&process, BROWSER_START_TIMEOUT + RENDER_TIMEOUT);
    let debug_ws_url = wait_for_debug_websocket(&mut process, startup_deadline)?;
    let remaining_startup = startup_deadline.saturating_duration_since(Instant::now());
    ensure!(
        !remaining_startup.is_zero(),
        "Chromium snapshot browser startup timed out before CDP connection"
    );
    let browser = Browser::connect_with_timeout(debug_ws_url, remaining_startup.min(CDP_TIMEOUT))
        .context("failed to connect to Chromium snapshot CDP")?;
    let render_deadline = Instant::now() + RENDER_TIMEOUT;
    browser.set_default_timeout(RENDER_TIMEOUT);
    let tab = browser.new_tab().context("failed to create Chromium snapshot tab")?;
    tab.set_default_timeout(remaining_until(
        render_deadline,
        "Chromium snapshot rendering timed out",
    )?);
    tab.call_method(Emulation::SetDeviceMetricsOverride {
        width: viewport_width_px,
        height: VIEWPORT_HEIGHT_PX,
        device_scale_factor: DEVICE_SCALE_FACTOR,
        mobile: false,
        scale: None,
        screen_width: Some(viewport_width_px),
        screen_height: Some(VIEWPORT_HEIGHT_PX),
        position_x: None,
        position_y: None,
        dont_set_visible_size: None,
        screen_orientation: None,
        viewport: None,
        display_feature: None,
        device_posture: None,
    })
    .context("failed to set Chromium device scale factor")?;
    tab.navigate_to(page_url)
        .context("failed to navigate Chromium snapshot tab")?
        .wait_until_navigated()
        .context("Chromium snapshot navigation timed out")?;

    let diagnostics = wait_for_readiness(&tab, viewport_width_px, render_deadline)?;
    remaining_until(render_deadline, "Chromium snapshot rendering timed out before capture")?;
    let screenshot = tab
        .call_method(Page::CaptureScreenshot {
            format: Some(Page::CaptureScreenshotFormatOption::Png),
            quality: None,
            clip: None,
            from_surface: Some(true),
            capture_beyond_viewport: Some(true),
            optimize_for_speed: Some(false),
        })
        .context("failed to capture full-page Chromium PNG snapshot")?
        .data;
    let png_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, screenshot)
        .context("Chromium returned an invalid PNG screenshot")?;
    ensure!(
        image::guess_format(&png_bytes).is_ok_and(|format| format == image::ImageFormat::Png),
        "Chromium snapshot did not produce a PNG raster"
    );

    Ok(SnapshotResult { png_bytes, diagnostics })
}

fn launch_browser(browser_path: &Path, viewport_width_px: u32) -> Result<ChromiumProcess> {
    let profile = tempfile::Builder::new()
        .prefix("mdv-chromium-profile-")
        .tempdir()
        .context("failed to create dedicated Chromium profile")?;
    let profile_arg = format!("--user-data-dir={}", profile.path().display());
    let window_size_arg = format!("--window-size={viewport_width_px},{VIEWPORT_HEIGHT_PX}");
    let child = Command::new(browser_path)
        .args([
            OsStr::new("--headless"),
            OsStr::new("--remote-debugging-port=0"),
            OsStr::new("--remote-debugging-address=127.0.0.1"),
            OsStr::new("--no-first-run"),
            OsStr::new("--no-default-browser-check"),
            OsStr::new(&profile_arg),
            OsStr::new(&window_size_arg),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| {
            format!("failed to launch Chromium snapshot browser: {}", browser_path.display())
        })?;
    Ok(ChromiumProcess { child: Arc::new(Mutex::new(child)), profile })
}

fn wait_for_debug_websocket(process: &mut ChromiumProcess, deadline: Instant) -> Result<String> {
    let active_port = process.profile.path().join("DevToolsActivePort");
    loop {
        if let Some(status) = process
            .child
            .lock()
            .map_err(|_| anyhow::anyhow!("Chromium snapshot browser child lock poisoned"))?
            .try_wait()
            .context("failed to inspect Chromium snapshot browser")?
        {
            bail!("Chromium exited before opening DevToolsActivePort: {status}");
        }
        if let Ok(contents) = fs::read_to_string(&active_port)
            && let Ok(debug_ws_url) = parse_debug_websocket(&contents)
        {
            return Ok(debug_ws_url);
        }
        ensure!(
            Instant::now() < deadline,
            "Chromium did not open DevToolsActivePort before the startup deadline"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn parse_debug_websocket(contents: &str) -> Result<String> {
    let mut lines = contents.lines();
    let port = lines
        .next()
        .context("DevToolsActivePort did not contain a debugging port")?
        .parse::<u16>()
        .context("DevToolsActivePort contained an invalid debugging port")?;
    let path = lines.next().context("DevToolsActivePort did not contain a WebSocket path")?;
    ensure!(path.starts_with('/'), "DevToolsActivePort contained an invalid WebSocket path");
    Ok(format!("ws://127.0.0.1:{port}{path}"))
}

fn wait_for_readiness(
    tab: &headless_chrome::Tab,
    viewport_width_px: u32,
    deadline: Instant,
) -> Result<SnapshotDiagnostics> {
    let mut last_height: Option<f64> = None;
    let mut stable_probe_count = 0;
    let mut latest = None;

    for attempt in 0..=MAX_READINESS_ATTEMPTS {
        ensure!(Instant::now() < deadline, "Chromium snapshot rendering timed out");
        let evaluation = tab
            .call_method(Runtime::Evaluate {
                expression: READINESS_PROBE.to_string(),
                return_by_value: Some(true),
                generate_preview: Some(false),
                silent: Some(false),
                await_promise: Some(false),
                include_command_line_api: Some(false),
                user_gesture: Some(false),
                object_group: None,
                context_id: None,
                throw_on_side_effect: None,
                timeout: None,
                disable_breaks: None,
                repl_mode: None,
                allow_unsafe_eval_blocked_by_csp: None,
                unique_context_id: None,
                serialization_options: None,
            })
            .context("failed to evaluate Chromium snapshot readiness")?;
        let evaluation_debug = format!("{evaluation:?}");
        let value = evaluation.result.value.context(format!(
            "Chromium snapshot readiness did not return a value: {evaluation_debug}"
        ))?;
        let probe = serde_json::from_value::<ReadinessProbe>(value)
            .context("Chromium snapshot readiness returned invalid diagnostics")?;
        let visuals_ready = probe.fonts_ready
            && probe.images_ready
            && probe.mermaids_ready
            && (probe.remote_images_ready || attempt >= 8);
        if visuals_ready && last_height.is_some_and(|height| (height - probe.height).abs() < 0.5) {
            stable_probe_count += 1;
        } else {
            stable_probe_count = 0;
        }
        last_height = Some(probe.height);

        if (visuals_ready && stable_probe_count >= 1) || attempt == MAX_READINESS_ATTEMPTS {
            ensure_local_assets_rendered(&probe.images)?;
            ensure!(
                probe.fonts_ready,
                "fonts did not become ready before the Chromium snapshot deadline"
            );
            ensure!(
                probe.mermaids_ready,
                "Mermaid diagrams did not become ready before the Chromium snapshot deadline"
            );
            ensure!(
                probe.images_ready,
                "local images did not become ready before the Chromium snapshot deadline"
            );
            ensure_raster_bounds(viewport_width_px, probe.height)?;
            return Ok(probe.diagnostics());
        }
        latest = Some(probe.diagnostics());
        thread::sleep(READINESS_INTERVAL);
    }

    Ok(latest.unwrap_or_default())
}

fn remaining_until(deadline: Instant, timeout_message: &str) -> Result<Duration> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    ensure!(!remaining.is_zero(), "{timeout_message}");
    Ok(remaining)
}

fn ensure_raster_bounds(viewport_width_px: u32, css_height_px: f64) -> Result<()> {
    ensure!(
        css_height_px.is_finite() && css_height_px > 0.0,
        "Chromium reported an invalid full-page height: {css_height_px}"
    );
    let width_px = u64::from(viewport_width_px) * DEVICE_SCALE_FACTOR_PX;
    let max_css_height_px = MAX_RASTER_DIMENSION_PX / DEVICE_SCALE_FACTOR_PX;
    ensure!(
        css_height_px.ceil() <= max_css_height_px as f64,
        "full-page PNG would exceed the {MAX_RASTER_DIMENSION_PX}-pixel dimension limit; split the document"
    );
    let height_px = (css_height_px.ceil() as u64)
        .checked_mul(DEVICE_SCALE_FACTOR_PX)
        .context("full-page PNG height overflowed while applying the device scale factor")?;
    ensure!(
        width_px <= MAX_RASTER_DIMENSION_PX && height_px <= MAX_RASTER_DIMENSION_PX,
        "full-page PNG would be {width_px}x{height_px} pixels, exceeding the {}-pixel dimension limit; split the document",
        MAX_RASTER_DIMENSION_PX
    );
    let pixels = width_px.saturating_mul(height_px);
    ensure!(
        pixels <= MAX_RASTER_PIXELS,
        "full-page PNG would contain {pixels} pixels, exceeding the {MAX_RASTER_PIXELS}-pixel limit; split the document"
    );
    Ok(())
}

fn ensure_local_assets_rendered(images: &[ProbeAsset]) -> Result<()> {
    let failures = images
        .iter()
        .filter(|image| {
            image.blocking
                && (!image.complete
                    || (!image.current_src.is_empty() && image.natural_width <= 0.0))
        })
        .map(|image| {
            let source = if image.source.is_empty() { &image.current_src } else { &image.source };
            format!(
                "image {source} failed to render (complete={}, naturalWidth={}, rendered={}x{})",
                image.complete, image.natural_width, image.rendered_width, image.rendered_height
            )
        })
        .collect::<Vec<_>>();
    if failures.is_empty() { Ok(()) } else { bail!(failures.join("; ")) }
}

fn create_workspace(base_dir: &Path) -> Result<tempfile::TempDir> {
    tempfile::Builder::new()
        .prefix(".mdv-chromium-")
        .tempdir_in(base_dir)
        .or_else(|_| tempfile::Builder::new().prefix("mdv-chromium-").tempdir())
        .context("failed to create Chromium snapshot workspace")
}

fn browser_executable() -> Result<PathBuf> {
    if let Some(path) = environment_browser("MDV_BROWSER")? {
        return Ok(path);
    }
    #[cfg(test)]
    if let Some(path) = environment_browser("MDV_TEST_BROWSER")? {
        return Ok(path);
    }

    #[cfg(windows)]
    {
        installed_windows_browser()
    }

    #[cfg(all(test, target_os = "macos"))]
    {
        let chrome = PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome");
        ensure!(chrome.is_file(), "no test Chromium browser found; set MDV_TEST_BROWSER");
        Ok(chrome)
    }

    #[cfg(not(any(windows, all(test, target_os = "macos"))))]
    bail!("Chromium snapshot rendering is only supported on Windows");
}

fn environment_browser(variable: &str) -> Result<Option<PathBuf>> {
    let Some(value) = std::env::var_os(variable) else {
        return Ok(None);
    };
    let path = PathBuf::from(value);
    ensure!(
        path.is_file(),
        "{variable} must name an existing browser executable: {}",
        path.display()
    );
    Ok(Some(path))
}

#[cfg(windows)]
fn installed_windows_browser() -> Result<PathBuf> {
    let roots = [
        std::env::var_os("ProgramFiles(x86)"),
        std::env::var_os("ProgramFiles"),
        std::env::var_os("LOCALAPPDATA"),
    ];
    let browsers = [
        ["Microsoft", "Edge", "Application", "msedge.exe"],
        ["Google", "Chrome", "Application", "chrome.exe"],
    ];
    for browser in browsers {
        for root in roots.iter().flatten() {
            let candidate =
                browser.iter().fold(PathBuf::from(root), |path, component| path.join(component));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    bail!("no supported browser found; set MDV_BROWSER to an installed Edge or Chrome executable")
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;
    use image::{Rgba, RgbaImage};

    fn file_url(path: &Path) -> Result<String> {
        Ok(Url::from_directory_path(path)
            .map_err(|_| anyhow::anyhow!("fixture directory cannot become file URL"))?
            .to_string())
    }

    fn rich_html(base_url: &str, font_url: &str, theme: &str) -> String {
        format!(
            r#"<!doctype html><html><head><base href="{base_url}"><style>
            @font-face {{ font-family: "Mona Sans VF"; src: url("{font_url}") format("woff2"); }}
            body {{ margin:0; background:{theme}; color:#f0f6fc; font-family:"Mona Sans VF", sans-serif; }}
            .spacer {{ height:1500px; }} .tail {{ height:48px; background:rgb(1,35,70); }}
            </style></head><body><h1>こんにちは raster</h1><strong>Fresh asset</strong>
            <img src="pixel image.png" width="24" height="24"><img src="icon image.svg" width="24" height="24">
            <img src="http://127.0.0.1:9/remote-badge.svg" width="24" height="24">
            <svg class="mdv-mermaid-diagram" viewBox="0 0 40 40"><rect width="40" height="40"/></svg>
            <div class="spacer"></div><div class="tail"></div></body></html>"#
        )
    }

    #[cfg_attr(not(windows), ignore = "requires Chrome; run explicitly with MDV_TEST_BROWSER")]
    #[test]
    fn chromium_renders_fresh_full_page_assets_at_device_scale_two() -> Result<()> {
        let root = tempfile::tempdir()?;
        let base = root.path().join("unicode space 日本語");
        fs::create_dir_all(&base)?;
        let font_url = format!(
            "data:font/woff2;base64,{}",
            base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                include_bytes!("../../render/assets/fonts/MonaSansVF.woff2"),
            )
        );
        fs::write(
            base.join("icon image.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24" fill="#52b788"/></svg>"##,
        )?;
        RgbaImage::from_pixel(1, 1, Rgba([220, 38, 38, 255])).save(base.join("pixel image.png"))?;

        let base_url = file_url(&base)?;
        let light = render_html_to_png(&rich_html(&base_url, &font_url, "#ffffff"), &base, 320)?;
        let dark = render_html_to_png(&rich_html(&base_url, &font_url, "#0d1117"), &base, 320)?;
        let light_image = image::load_from_memory(&light.png_bytes)?.to_rgba8();
        let dark_image = image::load_from_memory(&dark.png_bytes)?.to_rgba8();
        assert_eq!(light_image.width(), 640, "CSS width must render at deviceScaleFactor=2");
        assert!(light_image.height() > 3_000, "full-page PNG must include the tall tail");
        assert_eq!(
            dark_image.dimensions(),
            light_image.dimensions(),
            "themes must retain page geometry"
        );
        assert_eq!(
            light_image.get_pixel(light_image.width() / 2, light_image.height() - 4),
            &Rgba([1, 35, 70, 255]),
            "the bottom pixels must come from the final page element"
        );
        assert_ne!(
            light.png_bytes, dark.png_bytes,
            "light and dark pages must render independently"
        );
        assert!(
            light.diagnostics.fonts_ready && light.diagnostics.prose_font_ready,
            "embedded production font must reach readiness: {:?}",
            light.diagnostics
        );
        assert!(light.diagnostics.images_ready && light.diagnostics.mermaids_ready);
        assert_eq!(
            light.diagnostics.images.len(),
            3,
            "local PNG, local SVG, and remote badge are diagnosed"
        );

        RgbaImage::from_pixel(1, 1, Rgba([37, 99, 235, 255])).save(base.join("pixel image.png"))?;
        let refreshed =
            render_html_to_png(&rich_html(&base_url, &font_url, "#ffffff"), &base, 320)?;
        assert_ne!(
            light.png_bytes, refreshed.png_bytes,
            "a new render must read the current local image"
        );
        Ok(())
    }

    #[cfg_attr(not(windows), ignore = "requires Chrome; run explicitly with MDV_TEST_BROWSER")]
    #[test]
    fn chromium_reports_missing_local_images() -> Result<()> {
        let base = tempfile::tempdir()?;
        let html =
            format!(r#"<base href="{}"><img src="missing local.png">"#, file_url(base.path())?);
        let error = match render_html_to_png(&html, base.path(), 320) {
            Ok(_) => panic!("a missing local image must fail the render"),
            Err(error) => error,
        };
        assert!(
            error.to_string().contains("image missing local.png failed to render"),
            "missing local assets must fail with an actionable diagnostic: {error:#}"
        );
        Ok(())
    }

    #[test]
    fn full_page_bounds_rejects_extreme_heights_without_overflowing() {
        let error = ensure_raster_bounds(320, f64::MAX)
            .err()
            .unwrap_or_else(|| panic!("an extreme page height must be rejected"));
        assert!(
            error.to_string().contains("full-page PNG"),
            "the error must tell the user why capture was refused: {error:#}"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn silent_browser_startup_times_out_and_reaps_the_child() -> Result<()> {
        use std::{os::unix::fs::PermissionsExt, process::Command};

        let workspace = tempfile::tempdir()?;
        let script = workspace.path().join("silent-browser.sh");
        fs::write(&script, "#!/bin/sh\nwhile :; do sleep 1; done\n")?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700))?;

        let started = Instant::now();
        let mut process = launch_browser(&script, 320)?;
        let pid = process
            .child
            .lock()
            .map_err(|_| anyhow::anyhow!("silent browser child lock poisoned"))?
            .id()
            .to_string();
        let error = match wait_for_debug_websocket(
            &mut process,
            Instant::now() + Duration::from_millis(100),
        ) {
            Ok(_) => panic!("a browser that never opens DevTools must fail startup"),
            Err(error) => error,
        };
        drop(process);
        assert!(started.elapsed() < Duration::from_secs(2), "startup deadline was not enforced");
        assert!(error.to_string().contains("DevToolsActivePort"));

        let alive = Command::new("/bin/kill")
            .args(["-0", &pid])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("inspect silent browser child")?
            .success();
        assert!(!alive, "timed-out browser child must be reaped");
        Ok(())
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn watchdog_reaps_a_child_when_connection_work_cannot_return() -> Result<()> {
        use std::{os::unix::fs::PermissionsExt, process::Command};

        let workspace = tempfile::tempdir()?;
        let script = workspace.path().join("silent-browser.sh");
        fs::write(&script, "#!/bin/sh\nwhile :; do sleep 1; done\n")?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700))?;

        let process = launch_browser(&script, 320)?;
        let pid = process
            .child
            .lock()
            .map_err(|_| anyhow::anyhow!("silent browser child lock poisoned"))?
            .id()
            .to_string();
        let watchdog = ProcessWatchdog::arm(&process, Duration::from_millis(100));
        thread::sleep(Duration::from_millis(200));

        let alive = Command::new("/bin/kill")
            .args(["-0", &pid])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("inspect watchdog browser child")?
            .success();
        assert!(!alive, "watchdog must kill and reap the owned browser child");
        drop(watchdog);
        drop(process);
        Ok(())
    }
}
