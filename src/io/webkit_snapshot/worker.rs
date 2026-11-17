use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, DirBuilder, OpenOptions},
    hash::Hasher,
    io::{self, BufRead, BufReader, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::UnixStream,
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

use super::SNAPSHOT_HELPER;

pub(super) struct RenderOptions<'a> {
    pub(super) width: u32,
    pub(super) format: &'a str,
    pub(super) wait_for_remote: bool,
    pub(super) cancelled: Option<&'a AtomicBool>,
}

#[derive(Deserialize)]
struct Response {
    ok: bool,
    error: Option<String>,
}

pub(super) fn render(
    html_path: &Path,
    read_access_root: &Path,
    output_path: &Path,
    report_path: &Path,
    options: RenderOptions<'_>,
) -> Result<()> {
    let RenderOptions { width, format, wait_for_remote, cancelled } = options;
    check_cancelled(cancelled)?;
    let directory = worker_directory(wait_for_remote)?;
    let request = serde_json::json!({
        "html_path": html_path, "read_access_root": read_access_root,
        "output_path": output_path, "report_path": report_path, "width": width, "output_format": format, "timeout_ms": 25_000,
        "wait_for_remote": wait_for_remote,
    });
    // Idle shutdown can race a connection. A render is idempotent, so retry a closed connection once.
    let response = send(&directory, &request, cancelled).or_else(|error| {
        if error.downcast_ref::<io::Error>().is_some_and(|error| {
            matches!(
                error.kind(),
                io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::UnexpectedEof
            )
        }) {
            send(&directory, &request, cancelled)
        } else {
            Err(error)
        }
    })?;
    ensure!(
        response.ok,
        "snapshot failed: {}",
        response.error.as_deref().unwrap_or("unknown worker error")
    );
    Ok(())
}

fn send(
    directory: &Path,
    request: &serde_json::Value,
    cancelled: Option<&AtomicBool>,
) -> Result<Response> {
    check_cancelled(cancelled)?;
    let mut stream = connect(directory)?;
    check_cancelled(cancelled)?;
    stream.set_read_timeout(Some(Duration::from_millis(75)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut payload = serde_json::to_vec(request)?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    let mut line = Vec::new();
    let mut reader = BufReader::new(stream);
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        check_cancelled(cancelled)?;
        if Instant::now() >= deadline {
            bail!("snapshot worker response timed out");
        }
        match reader.fill_buf() {
            Ok([]) => break,
            Ok(buffer) => {
                let count = buffer
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .map_or(buffer.len(), |index| index + 1);
                let has_newline = buffer.get(count.saturating_sub(1)) == Some(&b'\n');
                line.extend_from_slice(&buffer[..count]);
                reader.consume(count);
                if has_newline || line.len() > 65536 {
                    break;
                }
            }
            Err(error)
                if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) =>
            {
                continue;
            }
            Err(error) => return Err(error.into()),
        }
    }
    let line = String::from_utf8(line).context("snapshot worker response was not UTF-8")?;
    if line.len() > 65536 {
        bail!("snapshot worker response exceeded 65536 bytes");
    }
    if line.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "snapshot worker closed the connection",
        )
        .into());
    }
    serde_json::from_str(&line).context("snapshot worker returned invalid diagnostics")
}

fn check_cancelled(cancelled: Option<&AtomicBool>) -> Result<()> {
    if cancelled.is_some_and(|token| token.load(Ordering::Acquire)) {
        bail!("snapshot rendering cancelled");
    }
    Ok(())
}

fn worker_directory(wait_for_remote: bool) -> Result<PathBuf> {
    let uid = rustix::process::geteuid().as_raw();
    let mut version = DefaultHasher::new();
    version.write(SNAPSHOT_HELPER);
    // macOS sockaddr_un paths are limited to 104 bytes; TMPDIR can already exceed that.
    // A slow image refresh must not occupy the worker used for first display.
    let mode = if wait_for_remote { "-remote" } else { "" };
    let directory = PathBuf::from(format!("/tmp/mdv-render-{uid}-{:016x}{mode}", version.finish()));
    match DirBuilder::new().mode(0o700).create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(error).context("failed to create private snapshot worker directory");
        }
    }
    let metadata = fs::symlink_metadata(&directory)?;
    ensure!(
        metadata.is_dir() && metadata.uid() == uid && metadata.permissions().mode() & 0o077 == 0,
        "snapshot worker directory must be private and owned by the current user: {}",
        directory.display()
    );
    Ok(directory)
}

fn connect(directory: &Path) -> Result<UnixStream> {
    let socket = directory.join("worker.sock");
    if let Ok(stream) = UnixStream::connect(&socket) {
        return Ok(stream);
    }
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .mode(0o600)
        .open(directory.join("startup.lock"))?;
    lock.lock().context("failed to lock snapshot worker startup")?;
    match UnixStream::connect(&socket) {
        Ok(stream) => return Ok(stream),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) => {}
        Err(error) => return Err(error).context("failed to connect to snapshot worker"),
    }
    if socket.exists() {
        fs::remove_file(&socket)?;
    }
    let helper = directory.join("helper");
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.as_file().set_permissions(fs::Permissions::from_mode(0o700))?;
    temporary.write_all(SNAPSHOT_HELPER)?;
    temporary.persist(&helper).context("failed to install bundled snapshot worker")?;
    let log_path = directory.join("worker.log");
    let log =
        OpenOptions::new().create(true).truncate(true).write(true).mode(0o600).open(&log_path)?;
    let mut child = Command::new(&helper)
        .process_group(0)
        .arg("--serve")
        .arg(&socket)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .context("failed to start bundled snapshot worker")?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(stream) = UnixStream::connect(&socket) {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(stream);
        }
        if let Some(status) = child.try_wait()? {
            bail!(
                "snapshot worker exited with {status}: {}",
                fs::read_to_string(&log_path).unwrap_or_default()
            );
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!(
                "snapshot worker did not open its socket: {}",
                fs::read_to_string(&log_path).unwrap_or_default()
            );
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
