use std::io;

#[cfg(unix)]
use crate::io::kitty_graphics::{encode_support_query, parse_support_reply};
#[cfg(unix)]
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
#[cfg(unix)]
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    io::Errno,
};
#[cfg(unix)]
use std::{
    io::Write,
    time::{Duration, Instant},
};

#[cfg(unix)]
const REPLY_TIMEOUT: Duration = Duration::from_secs(1);

/// Whether the terminal answers a Kitty graphics query with `OK`.
#[cfg(unix)]
pub fn supports_kitty_graphics() -> io::Result<bool> {
    enable_raw_mode()?;
    let supported = query_kitty_graphics();
    disable_raw_mode()?;
    supported
}

/// Non-Unix images use the existing terminal-specific transport.
#[cfg(not(unix))]
pub fn supports_kitty_graphics() -> io::Result<bool> {
    Ok(false)
}

#[cfg(unix)]
fn query_kitty_graphics() -> io::Result<bool> {
    let mut stdout = io::stdout();
    stdout.write_all(encode_support_query().as_bytes())?;
    stdout.flush()?;

    let stdin = io::stdin();
    let mut input = [PollFd::new(&stdin, PollFlags::IN)];
    let deadline = Instant::now() + REPLY_TIMEOUT;
    let mut reply = Vec::new();
    let mut byte = [0; 1];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(false);
        }
        let timeout = Timespec::try_from(remaining).map_err(io::Error::other)?;
        match poll(&mut input, Some(&timeout)) {
            Ok(0) => return Ok(false),
            Ok(_) => {}
            Err(Errno::INTR) => continue,
            Err(error) => return Err(error.into()),
        }

        // Bypass buffered Stdin reads: bytes after DA1 must stay on the tty for crossterm.
        match rustix::io::read(&stdin, &mut byte) {
            Ok(0) => return Ok(false),
            Ok(_) => reply.extend_from_slice(&byte),
            Err(Errno::INTR | Errno::AGAIN) => continue,
            Err(error) => return Err(error.into()),
        }
        if let Some(supported) = parse_support_reply(&reply) {
            return Ok(supported);
        }
    }
}
