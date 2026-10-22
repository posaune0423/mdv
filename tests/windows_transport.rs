#![cfg(any(target_os = "macos", target_os = "windows"))]

use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use image::{ImageBuffer, Rgba};
use portable_pty::{Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

struct ViewerSession {
    child: Box<dyn Child + Send + Sync>,
    master: Option<Box<dyn MasterPty + Send>>,
    input: Box<dyn Write + Send>,
    output: Receiver<Vec<u8>>,
    bytes: Vec<u8>,
    scan: usize,
    #[cfg(target_os = "windows")]
    conpty_cursor_query_replied: bool,
}

impl ViewerSession {
    fn start(document: &std::path::Path) -> Self {
        let pair = NativePtySystem::default()
            .openpty(size(120, 40))
            .unwrap_or_else(|error| panic!("native PTY should open: {error}"));
        let mut reader = pair
            .master
            .try_clone_reader()
            .unwrap_or_else(|error| panic!("PTY output should open: {error}"));
        let input = pair
            .master
            .take_writer()
            .unwrap_or_else(|error| panic!("PTY input should open: {error}"));
        let (sender, output) = mpsc::sync_channel(8);
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 65536];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        if sender.send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_mdv"));
        command.arg(document);
        command.env("TERM_PROGRAM", "WezTerm");
        command.env("TERM", "xterm-256color");
        command.env("RUST_LOG", "off");
        let child = pair
            .slave
            .spawn_command(command)
            .unwrap_or_else(|error| panic!("native viewer should start: {error}"));
        drop(pair.slave);
        let session = Self {
            child,
            master: Some(pair.master),
            input,
            output,
            bytes: Vec::new(),
            scan: 0,
            #[cfg(target_os = "windows")]
            conpty_cursor_query_replied: false,
        };
        #[cfg(target_os = "windows")]
        let session = {
            let mut session = session;
            session.reply_to_conpty_cursor_query();
            session
        };
        session
    }

    #[cfg(target_os = "windows")]
    fn reply_to_conpty_cursor_query(&mut self) {
        const QUERY: &[u8] = b"\x1b[6n";
        const RESPONSE: &[u8] = b"\x1b[1;1R";
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if self.bytes.windows(QUERY.len()).any(|part| part == QUERY) {
                self.send(RESPONSE);
                self.conpty_cursor_query_replied = true;
                return;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "ConPTY did not request its inherited cursor position before startup; raw output={}",
                escaped_tail(&self.bytes)
            );
            match self.output.recv_timeout(remaining) {
                Ok(data) => self.bytes.extend(data),
                Err(mpsc::RecvTimeoutError::Timeout) => panic!(
                    "ConPTY did not request its inherited cursor position before startup; raw output={}",
                    escaped_tail(&self.bytes)
                ),
                Err(error) => panic!(
                    "ConPTY output closed before its inherited cursor query: {error}; raw output={}",
                    escaped_tail(&self.bytes)
                ),
            }
        }
    }

    fn send(&mut self, input: &[u8]) {
        self.input.write_all(input).unwrap_or_else(|error| panic!("input should send: {error}"));
        self.input.flush().unwrap_or_else(|error| panic!("input should flush: {error}"));
    }

    fn next_png(&mut self) -> Vec<u8> {
        const PREFIX: &[u8] = b"\x1b]1337;File=";
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            if let Some(start) = self.bytes.windows(PREFIX.len()).position(|part| part == PREFIX) {
                if start > 0 {
                    self.bytes.drain(..start);
                    self.scan = 0;
                }
                let start_scan = self.scan.max(PREFIX.len());
                let end = (start_scan..self.bytes.len()).find_map(|index| {
                    if self.bytes[index] == 7 {
                        Some((index, 1))
                    } else if self.bytes[index..].starts_with(b"\x1b\\") {
                        Some((index, 2))
                    } else {
                        None
                    }
                });
                if let Some((end, terminator)) = end {
                    let packet = self.bytes[..end].to_vec();
                    self.bytes.drain(..end + terminator);
                    self.scan = 0;
                    let separator = packet
                        .iter()
                        .position(|byte| *byte == b':')
                        .unwrap_or_else(|| panic!("inline image should contain a payload"));
                    return STANDARD.decode(&packet[separator + 1..]).unwrap_or_else(|error| {
                        panic!("ConPTY must preserve the complete image: {error}")
                    });
                }
                self.scan = self.bytes.len().saturating_sub(1);
            } else if self.bytes.len() > PREFIX.len() {
                self.bytes.drain(..self.bytes.len() - PREFIX.len());
                self.scan = 0;
            }
            assert!(Instant::now() < deadline, "viewer produced no inline PNG before deadline");
            match self.output.recv_timeout(Duration::from_secs(1)) {
                Ok(data) => self.bytes.extend(data),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(error) => panic!(
                    "viewer must produce rich image output, not text fallback: {error}; raw output={}",
                    escaped_tail(&self.bytes)
                ),
            }
            assert!(self.bytes.len() < 32 * 1024 * 1024, "inline image exceeded the fixture bound");
        }
    }

    fn quit(&mut self) {
        self.send(b"q");
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self
                .child
                .try_wait()
                .unwrap_or_else(|error| panic!("viewer status should read: {error}"))
            {
                assert!(status.success(), "viewer should exit successfully: {status:?}");
                return;
            }
            assert!(Instant::now() < deadline, "viewer did not quit");
            let _ = self.output.recv_timeout(Duration::from_millis(20));
        }
    }
}

impl Drop for ViewerSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        #[cfg(target_os = "windows")]
        if !self.conpty_cursor_query_replied
            && let Some(master) = self.master.take()
        {
            // ClosePseudoConsole can wait forever for this reply and hide the startup panic.
            // The failed test process will release the handle when it exits.
            std::mem::forget(master);
        }
    }
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize { rows, cols, pixel_width: cols * 8, pixel_height: rows * 16 }
}

fn escaped_tail(bytes: &[u8]) -> String {
    const LIMIT: usize = 512;
    bytes[bytes.len().saturating_sub(LIMIT)..]
        .iter()
        .flat_map(|byte| std::ascii::escape_default(*byte))
        .map(char::from)
        .collect()
}

#[test]
fn native_pty_preserves_rich_png_scrolling_resize_and_quit() {
    let directory = tempfile::Builder::new()
        .prefix("mdv Unicode 日本語 ")
        .tempdir()
        .unwrap_or_else(|error| panic!("fixture directory should create: {error}"));
    let mut random = 0x1234_5678_u32;
    let noise = ImageBuffer::from_fn(1200, 800, |_, _| {
        let mut channel = || {
            random ^= random << 13;
            random ^= random >> 17;
            random ^= random << 5;
            random as u8
        };
        Rgba([channel(), channel(), channel(), 255])
    });
    noise
        .save(directory.path().join("画像 with spaces.png"))
        .unwrap_or_else(|error| panic!("fixture image should save: {error}"));
    let document = directory.path().join("document 日本語.md");
    std::fs::write(&document, "# Native Windows rendering\n\n![lossless local image](画像%20with%20spaces.png)\n\n**Bottom content**\n\n".repeat(3))
        .unwrap_or_else(|error| panic!("document should write: {error}"));
    let opened = Instant::now();
    let mut viewer = ViewerSession::start(&document);
    let first_png = viewer.next_png();
    let first_png_ms = opened.elapsed().as_secs_f64() * 1000.0;
    if let Some(artifacts) = std::env::var_os("MDV_TEST_ARTIFACTS") {
        let artifacts = std::path::PathBuf::from(artifacts);
        std::fs::create_dir_all(&artifacts)
            .unwrap_or_else(|error| panic!("evidence directory should create: {error}"));
        std::fs::write(artifacts.join("native-pty-first-viewport.png"), &first_png)
            .unwrap_or_else(|error| panic!("native viewport evidence should save: {error}"));
    }
    let first = image::load_from_memory(&first_png)
        .unwrap_or_else(|error| panic!("native transport must deliver a complete PNG: {error}"));
    println!(
        "native inline viewport: {}x{}, {} PNG bytes",
        first.width(),
        first.height(),
        first_png.len()
    );
    let (minimum_width, minimum_height, minimum_bytes) =
        if cfg!(windows) { (960, 600, 512 * 1024) } else { (600, 380, 128 * 1024) };
    assert!(first.width() >= minimum_width && first.height() >= minimum_height);
    assert!(
        first_png.len() > minimum_bytes,
        "fixture must exercise a large lossless image payload"
    );
    let scroll_started = Instant::now();
    viewer.send(b"j");
    let scroll_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if viewer.next_png() != first_png {
            break;
        }
        assert!(Instant::now() < scroll_deadline, "scrolling must change the visible source crop");
    }
    let changed_scroll_ms = scroll_started.elapsed().as_secs_f64() * 1000.0;
    if let Some(artifacts) = std::env::var_os("MDV_TEST_ARTIFACTS") {
        let evidence = serde_json::json!({
            "platform": std::env::consts::OS,
            "terminal_transport": "WezTerm OSC1337 through native PTY",
            "fixture": "Unicode and space paths, local random PNG, repeated rich Markdown",
            "first_png_ms": first_png_ms,
            "first_changed_scroll_png_ms": changed_scroll_ms,
            "first_png_bytes": first_png.len(),
            "first_png_dimensions": [first.width(), first.height()],
            "boundary": "PTY receive plus base64 decode; native terminal compositor excluded",
        });
        std::fs::write(
            std::path::PathBuf::from(artifacts).join("native-pty-evidence.json"),
            serde_json::to_vec_pretty(&evidence)
                .unwrap_or_else(|error| panic!("evidence should serialize: {error}")),
        )
        .unwrap_or_else(|error| panic!("evidence should save: {error}"));
    }
    viewer
        .master
        .as_ref()
        .unwrap_or_else(|| panic!("native PTY should remain open until after resize"))
        .resize(size(100, 35))
        .unwrap_or_else(|error| panic!("native console resize should work: {error}"));
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let resized = image::load_from_memory(&viewer.next_png())
            .unwrap_or_else(|error| panic!("resized viewport must remain lossless PNG: {error}"));
        if resized.width() < first.width() {
            break;
        }
        assert!(Instant::now() < deadline, "resize must change raster geometry");
    }
    viewer.quit();
}
