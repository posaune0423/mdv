#![cfg(unix)]

use std::{
    io::{Read, Write},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled},
};
use mdv::{io::kitty_graphics::encode_support_query, ui::terminal::probe::supports_kitty_graphics};
use portable_pty::{Child, CommandBuilder, NativePtySystem, PtySize, PtySystem};

struct ProbeSession {
    child: Box<dyn Child + Send + Sync>,
    input: Box<dyn Write + Send>,
    output: Receiver<Vec<u8>>,
    bytes: Vec<u8>,
}

impl ProbeSession {
    fn start(mode: &str) -> Self {
        let pair = NativePtySystem::default()
            .openpty(PtySize { rows: 24, cols: 80, pixel_width: 640, pixel_height: 384 })
            .unwrap_or_else(|error| panic!("PTY should open: {error}"));
        let mut reader = pair.master.try_clone_reader().unwrap_or_else(|error| panic!("{error}"));
        let input = pair.master.take_writer().unwrap_or_else(|error| panic!("{error}"));
        let (sender, output) = mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = [0; 1024];
            while let Ok(count @ 1..) = reader.read(&mut bytes) {
                if sender.send(bytes[..count].to_vec()).is_err() {
                    break;
                }
            }
        });
        let mut command = CommandBuilder::new(
            std::env::current_exe()
                .unwrap_or_else(|error| panic!("test executable should resolve: {error}")),
        );
        command.args(["--exact", "probe_child", "--ignored", "--nocapture"]);
        command.env("MDV_PROBE_TEST_MODE", mode);
        let child = pair.slave.spawn_command(command).unwrap_or_else(|error| panic!("{error}"));
        Self { child, input, output, bytes: Vec::new() }
    }

    fn wait_for(&mut self, marker: &[u8]) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.bytes.windows(marker.len()).any(|bytes| bytes == marker) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let bytes = self.output.recv_timeout(remaining).unwrap_or_else(|error| {
                panic!(
                    "missing {marker:?}: {error}; output: {}",
                    String::from_utf8_lossy(&self.bytes)
                )
            });
            self.bytes.extend(bytes);
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.input.write_all(bytes).unwrap_or_else(|error| panic!("{error}"));
        self.input.flush().unwrap_or_else(|error| panic!("{error}"));
    }
}

impl Drop for ProbeSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn probe_preserves_key_after_device_attributes() {
    let mut session = ProbeSession::start("key");
    session.wait_for(encode_support_query().as_bytes());
    session.send(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;22cq");
    session.wait_for(b"KEY_PRESERVED");
}

#[test]
fn probe_timeout_releases_stdin() {
    let mut session = ProbeSession::start("timeout");
    session.wait_for(encode_support_query().as_bytes());
    session.wait_for(b"STDIN_RELEASED");
}

#[test]
#[ignore = "subprocess entry point for the PTY tests"]
fn probe_child() {
    let mode = std::env::var("MDV_PROBE_TEST_MODE")
        .unwrap_or_else(|error| panic!("PTY test mode should be set: {error}"));
    let supported =
        supports_kitty_graphics().unwrap_or_else(|error| panic!("probe failed: {error}"));
    assert!(!is_raw_mode_enabled().unwrap_or_else(|error| panic!("{error}")));
    if mode == "timeout" {
        assert!(!supported);
        // The old timed-out reader retains this lock forever.
        drop(std::io::stdin().lock());
        println!("STDIN_RELEASED");
        return;
    }

    assert!(supported);
    enable_raw_mode().unwrap_or_else(|error| panic!("{error}"));
    let key = if event::poll(Duration::from_millis(500)).unwrap_or_else(|error| panic!("{error}")) {
        Some(event::read().unwrap_or_else(|error| panic!("{error}")))
    } else {
        None
    };
    disable_raw_mode().unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(key, Some(Event::Key(key)) if key.code == KeyCode::Char('q')));
    println!("KEY_PRESERVED");
}
