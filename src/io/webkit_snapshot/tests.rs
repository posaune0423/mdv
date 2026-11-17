use std::path::Path;

use super::{
    SnapshotDiagnostics,
    paths::{cleanup_workspace, create_workspace, snapshot_asset_root},
};

#[cfg(target_os = "macos")]
#[test]
fn single_snapshot_apis_include_delayed_remote_images() -> anyhow::Result<()> {
    use std::{
        io::{Read, Write},
        time::{Duration, Instant},
    };
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?;
    let server = std::thread::spawn(move || -> anyhow::Result<()> {
        for _ in 0..2 {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        anyhow::ensure!(Instant::now() < deadline, "image request timed out");
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => return Err(error.into()),
                }
            };
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            let mut request = [0; 4096];
            let received = stream.read(&mut request)?;
            anyhow::ensure!(received > 0, "image client closed without a request");
            std::thread::sleep(Duration::from_millis(600));
            let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24'><path fill='red' d='M0 0h24v24H0z'/></svg>";
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: image/svg+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                svg.len(),
                svg
            );
        }
        Ok(())
    });
    let workspace = tempfile::tempdir()?;
    let html = format!("<body><img src='http://{address}/delayed.svg'></body>");
    let raster = crate::io::snapshot::render_html_to_raster(&html, workspace.path(), 320);
    let png = super::render_html_to_png(&html, workspace.path(), 320);
    server.join().map_err(|_| anyhow::anyhow!("image server panicked"))??;
    for (name, bytes) in [("raster", raster?), ("PNG", png?.png_bytes)] {
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        anyhow::ensure!(
            image.pixels().any(|pixel| pixel.0 == [255, 0, 0, 255]),
            "single {name} snapshot must include the delayed remote image"
        );
    }
    Ok(())
}

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
        io::{BufRead, BufReader, Read, Write},
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
        let render_with_remote =
            |format: &str, timeout_ms: u32, wait_for_remote: bool| -> Result<serde_json::Value> {
                let mut request = request.clone();
                request["output_format"] = format.into();
                request["timeout_ms"] = timeout_ms.into();
                request["wait_for_remote"] = wait_for_remote.into();
                let mut stream = UnixStream::connect(&socket)?;
                stream.set_read_timeout(Some(Duration::from_secs(15)))?;
                serde_json::to_writer(&mut stream, &request)?;
                stream.write_all(b"\n")?;
                let mut line = String::new();
                BufReader::new(stream).read_line(&mut line)?;
                serde_json::from_str(&line).context("worker response must be JSON")
            };
        let render = |format, timeout_ms| render_with_remote(format, timeout_ms, false);
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

        // Hold an HTTP image response open until after the render deadline. The
        // local document must still render, with diagnostics for the pending image.
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let (release, released) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let mut connections = Vec::new();
            loop {
                if let Ok((stream, _)) = listener.accept() {
                    connections.push(stream);
                }
                if !matches!(
                    released.recv_timeout(Duration::from_millis(10)),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                ) {
                    return connections.len();
                }
            }
        });
        std::fs::write(
            &html,
            format!(
                "<body style='margin:0;background:#123456'><h1>Readable</h1>\
                 <img src='http://{address}/slow.png' width='24' height='24'></body>"
            ),
        )?;
        let pending = render("png", 2_000);
        let _ = release.send(());
        let connections = server.join().map_err(|_| anyhow::anyhow!("image server panicked"))?;
        ensure!(connections > 0, "WebKit must actually request the slow remote image");
        ensure!(
            pending?["ok"] == true,
            "a pending remote image must not block the local page until the render deadline"
        );
        let diagnostics: SnapshotDiagnostics = serde_json::from_slice(&std::fs::read(&report)?)?;
        ensure!(diagnostics.images.len() == 1, "pending image must remain in diagnostics");
        ensure!(!diagnostics.images[0].complete, "the remote response must still be pending");
        let pixels = image::open(&output)?.to_rgba8();
        ensure!(
            pixels.get_pixel(319, 0).0 == [0x12, 0x34, 0x56, 255],
            "the snapshot must contain the requested document, not the previous page"
        );

        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let (release, released) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let mut connections = Vec::new();
            let mut first_request = None;
            loop {
                if let Ok((mut stream, _)) = listener.accept() {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let mut request = [0; 4096];
                    let _ = stream.read(&mut request);
                    first_request.get_or_insert_with(Instant::now);
                    connections.push(stream);
                }
                if first_request.is_some_and(|time| time.elapsed() >= Duration::from_millis(600)) {
                    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24'><path fill='red' d='M0 0h24v24H0z'/></svg>";
                    for mut stream in connections.drain(..) {
                        let _ = write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Type: image/svg+xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            svg.len(),
                            svg
                        );
                    }
                }
                if !matches!(
                    released.recv_timeout(Duration::from_millis(10)),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                ) {
                    return;
                }
            }
        });
        std::fs::write(&html, format!("<body><img src='http://{address}/delayed.svg'></body>"))?;
        let complete = render_with_remote("png", 2_000, true);
        let _ = release.send(());
        server.join().map_err(|_| anyhow::anyhow!("image server panicked"))?;
        ensure!(complete?["ok"] == true, "background render must complete");
        let diagnostics: SnapshotDiagnostics = serde_json::from_slice(&std::fs::read(&report)?)?;
        ensure!(
            diagnostics.images[0].complete && diagnostics.images[0].natural_width_px == 24.0,
            "background render must include a remote image that arrives after the first-frame budget: {diagnostics:?}"
        );
        ensure!(
            image::open(&output)?.to_rgba8().pixels().any(|pixel| pixel.0 == [255, 0, 0, 255]),
            "completed remote image pixels must appear in the final snapshot"
        );

        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let (requested, was_requested) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let mut connections = Vec::new();
            let mut reported = false;
            loop {
                if let Ok((stream, _)) = listener.accept() {
                    if !reported {
                        reported = true;
                        let _ = requested.send(());
                    }
                    connections.push(stream);
                }
                if !matches!(
                    released.recv_timeout(Duration::from_millis(10)),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                ) {
                    return;
                }
            }
        });
        std::fs::write(&html, format!("<body><img src='http://{address}/never.png'></body>"))?;
        let mut cancelled_request = request.clone();
        cancelled_request["output_format"] = "png".into();
        cancelled_request["timeout_ms"] = 25_000.into();
        cancelled_request["wait_for_remote"] = true.into();
        let mut cancelled_stream = UnixStream::connect(&socket)?;
        serde_json::to_writer(&mut cancelled_stream, &cancelled_request)?;
        cancelled_stream.write_all(b"\n")?;
        was_requested.recv_timeout(Duration::from_secs(5))?;
        drop(cancelled_stream);
        ensure!(
            render("png", 2_000)?["ok"] == true,
            "the worker must accept a new render after its client cancels a pending remote image"
        );
        let _ = release.send(());
        server.join().map_err(|_| anyhow::anyhow!("image server panicked"))?;
        Ok(())
    })();
    let _ = child.kill();
    let _ = child.wait();
    result.unwrap_or_else(|error| panic!("persistent renderer contract failed: {error:#}"));
}
