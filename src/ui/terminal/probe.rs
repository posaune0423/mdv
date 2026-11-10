use std::{
    io::{self, Read, Write},
    sync::mpsc,
    thread,
    time::Duration,
};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use crate::io::kitty_graphics::{encode_support_query, parse_support_reply};

const REPLY_TIMEOUT: Duration = Duration::from_secs(1);

/// Whether the terminal answers a Kitty graphics query with `OK`.
pub fn supports_kitty_graphics() -> io::Result<bool> {
    // Windows images go through WezTerm's inline transport, not Kitty graphics.
    if cfg!(windows) {
        return Ok(false);
    }

    enable_raw_mode()?;
    let supported = query_kitty_graphics();
    disable_raw_mode()?;
    supported
}

fn query_kitty_graphics() -> io::Result<bool> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new().spawn(move || {
        let mut stdin = io::stdin().lock();
        let mut reply = Vec::new();
        let mut byte = [0; 1];
        while matches!(stdin.read(&mut byte), Ok(1)) {
            reply.extend_from_slice(&byte);
            if let Some(supported) = parse_support_reply(&reply) {
                let _ = sender.send(supported);
                return;
            }
        }
    })?;

    let mut stdout = io::stdout();
    stdout.write_all(encode_support_query().as_bytes())?;
    stdout.flush()?;

    Ok(receiver.recv_timeout(REPLY_TIMEOUT).unwrap_or(false))
}
