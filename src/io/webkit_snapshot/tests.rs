use std::path::Path;

use super::{
    SnapshotDiagnostics,
    paths::{cleanup_workspace, create_workspace, snapshot_asset_root},
};

#[cfg(target_os = "macos")]
#[test]
fn platform_raster_facade_preserves_lossless_graphics_input() {
    let workspace = tempfile::tempdir().unwrap_or_else(|error| panic!("workspace: {error}"));
    let raster = crate::io::snapshot::render_html_to_raster(
        "<body style='margin:0;background:#243b53'><h1>Raster</h1></body>",
        workspace.path(),
        320,
    )
    .unwrap_or_else(|error| panic!("platform raster facade should render: {error:#}"));

    assert!(
        image::load_from_memory(&raster).is_ok(),
        "the graphics pipeline must receive a decodable lossless raster"
    );
}

#[test]
fn diagnostics_default_to_empty_assets() {
    let diagnostics = SnapshotDiagnostics::default();

    assert!(diagnostics.images.is_empty());
    assert!(diagnostics.mermaids.is_empty());
}

#[test]
fn snapshot_asset_root_expands_to_cover_relative_parent_assets() {
    let html = r#"<html><body><img src="../../../../examples/pixel.png" /></body></html>"#;
    let project = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("project fixture should be created: {error}"));
    let base_dir = project.path().join("tests/fixtures/gfm/image");
    std::fs::create_dir_all(&base_dir)
        .unwrap_or_else(|error| panic!("fixture base directory should be created: {error}"));

    let root = snapshot_asset_root(html, &base_dir);

    assert_eq!(root, project.path());
}

#[test]
fn snapshot_workspace_uses_temp_dir() {
    let temp_dir = tempfile::tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace root should be created: {error}"));
    let workspace = create_workspace(temp_dir.path(), temp_dir.path())
        .unwrap_or_else(|error| panic!("workspace should be created: {error}"));

    assert!(workspace.starts_with(temp_dir.path()));
    assert_eq!(
        workspace.parent().and_then(Path::file_name),
        Some(std::ffi::OsStr::new(".mdv-webkit"))
    );
    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn cleanup_workspace_removes_empty_mdv_webkit_parent() {
    let base = std::env::temp_dir().join(format!(
        "mdv-webkit-cleanup-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir_all(&base)
        .unwrap_or_else(|error| panic!("base directory should be created: {error}"));
    let workspace = create_workspace(&base, &base)
        .unwrap_or_else(|error| panic!("workspace should be created: {error}"));
    let parent = workspace
        .parent()
        .unwrap_or_else(|| panic!("workspace should have a parent"))
        .to_path_buf();

    cleanup_workspace(&workspace)
        .unwrap_or_else(|error| panic!("workspace cleanup should succeed: {error}"));

    assert!(!parent.exists(), "empty .mdv-webkit parent should be removed");
    let _ = std::fs::remove_dir_all(base);
}

#[cfg(target_os = "macos")]
#[test]
fn worker_renders_fresh_documents_and_recovers_after_asset_errors() {
    use anyhow::{Context, Result, ensure};
    use std::{
        io::{BufRead, BufReader, Write},
        os::unix::{fs::PermissionsExt, net::UnixStream},
        process::{Command, Stdio},
        time::{Duration, Instant},
    };

    let dir = tempfile::Builder::new()
        .prefix("mdv-worker-test-")
        .tempdir_in("/tmp")
        .unwrap_or_else(|error| panic!("worker workspace: {error}"));
    let helper = dir.path().join("helper");
    std::fs::write(&helper, super::SNAPSHOT_HELPER)
        .unwrap_or_else(|error| panic!("write helper: {error}"));
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700))
        .unwrap_or_else(|error| panic!("helper permissions: {error}"));
    let socket = dir.path().join("worker.sock");
    let mut child = Command::new(&helper)
        .arg("--serve")
        .arg(&socket)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("start worker: {error}"));

    let result = (|| -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if UnixStream::connect(&socket).is_ok() {
                break;
            }
            ensure!(
                child.try_wait()?.is_none(),
                "worker exited before accepting rendering requests"
            );
            ensure!(Instant::now() < deadline, "worker did not start");
            std::thread::sleep(Duration::from_millis(10));
        }
        let html = dir.path().join("document.html");
        let output = dir.path().join("snapshot.png");
        let report = dir.path().join("report.json");
        let request = serde_json::json!({
            "html_path": html, "read_access_root": dir.path(),
            "output_path": output, "report_path": report, "width": 320,
        });
        let render = |format: &str, timeout_ms: u32| -> Result<serde_json::Value> {
            let mut request = request.clone();
            request["output_format"] = format.into();
            request["timeout_ms"] = timeout_ms.into();
            let mut stream = UnixStream::connect(&socket)?;
            stream.set_read_timeout(Some(Duration::from_secs(15)))?;
            serde_json::to_writer(&mut stream, &request)?;
            stream.write_all(b"\n")?;
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line)?;
            serde_json::from_str(&line).context("worker response must be JSON")
        };
        for color in ["#ff0000", "#0000ff"] {
            std::fs::write(
                &html,
                format!("<body style='margin:0;background:{color}'><h1>Fresh</h1></body>"),
            )?;
            ensure!(render("png", 25_000)?["ok"] == true, "worker should render updated HTML");
            let actual = image::open(&output)?.to_rgba8();
            let reference = dir.path().join("reference.png");
            let status = Command::new(&helper)
                .args([
                    html.as_os_str(),
                    dir.path().as_os_str(),
                    reference.as_os_str(),
                    report.as_os_str(),
                    std::ffi::OsStr::new("320"),
                ])
                .status()?;
            ensure!(status.success(), "reference render should succeed");
            ensure!(
                actual == image::open(reference)?.to_rgba8(),
                "worker must preserve exact reference pixels after document changes"
            );
        }
        ensure!(
            render("tiff", 25_000)?["ok"] == true,
            "worker should return lossless raster transport"
        );
        let tiff = std::fs::read(&output)?;
        ensure!(
            image::guess_format(&tiff)? == image::ImageFormat::Tiff,
            "worker should avoid full-page PNG compression when TIFF is requested"
        );
        ensure!(
            image::load_from_memory(&tiff)?.to_rgba8()
                == image::open(dir.path().join("reference.png"))?.to_rgba8(),
            "lossless transport must preserve reference RGBA pixels"
        );
        std::fs::write(&html, "<img src='missing.png'>")?;
        let failure = render("png", 25_000)?;
        ensure!(
            failure["ok"] == false
                && failure["error"]
                    .as_str()
                    .is_some_and(|error| error.contains("failed to render")),
            "broken image diagnostics must survive worker mode: {failure}"
        );
        std::fs::write(&html, "<body><h1>Recovered</h1></body>")?;
        ensure!(
            render("png", 25_000)?["ok"] == true,
            "worker should recover after a failed document"
        );
        let timed_out = render("png", 1)?;
        ensure!(
            timed_out["ok"] == false
                && timed_out["error"].as_str().is_some_and(|error| error.contains("timed out")),
            "a render deadline must cancel active work: {timed_out}"
        );
        ensure!(
            render("png", 25_000)?["ok"] == true,
            "worker must render the next document after cancellation"
        );
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap_or_else(|error| panic!("persistent renderer contract failed: {error:#}"));
}
