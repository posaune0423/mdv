use std::process::Command;

use anyhow::{Context, Result, bail};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaneGeometry {
    pub column: u16,
    pub row: u16,
    pub status_top_rows: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TmuxConfig {
    pub geometry: PaneGeometry,
    pub input_buffer_size: usize,
}

pub fn load_config(pane: &str) -> Result<TmuxConfig> {
    let geometry = run_tmux([
        "display-message",
        "-p",
        "-t",
        pane,
        "#{pane_left},#{pane_top},#{status},#{status-position}",
    ])
    .and_then(|output| parse_pane_geometry(&output))?;
    let passthrough = run_tmux(["display-message", "-p", "-t", pane, "#{allow-passthrough}"])?;
    if passthrough.trim() != "on" {
        bail!("tmux allow-passthrough must be on; run: tmux set-option -g allow-passthrough on");
    }
    let input_buffer_size = run_tmux(["show-options", "-sv", "input-buffer-size"])
        .and_then(|output| parse_input_buffer_size(&output))?;
    Ok(TmuxConfig { geometry, input_buffer_size })
}

fn run_tmux<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("tmux").args(args).output().context("failed to query tmux")?;
    if !output.status.success() {
        bail!("tmux query failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    String::from_utf8(output.stdout).context("tmux returned non-UTF-8 output")
}

pub fn parse_pane_geometry(output: &str) -> Result<PaneGeometry> {
    let mut fields = output.trim().split(',');
    let column = fields.next().context("tmux pane geometry must contain a column")?;
    let row = fields.next().context("tmux pane geometry must contain a row")?;
    let status = fields.next().context("tmux pane geometry must contain status state")?;
    let status_position =
        fields.next().context("tmux pane geometry must contain status position")?;
    if fields.next().is_some() {
        bail!("tmux pane geometry has unexpected fields");
    }
    let status_top_rows = u16::from(status == "on" && status_position == "top");
    Ok(PaneGeometry {
        column: column.parse().context("tmux pane column is invalid")?,
        // pane_top excludes a top status row. With an attached tmux 3.7c
        // client, pane_top=0 and a row-0 marker rendered at CUP 2;1.
        row: row
            .parse::<u16>()
            .context("tmux pane row is invalid")?
            .saturating_add(status_top_rows),
        status_top_rows,
    })
}

pub fn parse_input_buffer_size(output: &str) -> Result<usize> {
    let size = output.trim().parse().context("tmux input-buffer-size is invalid")?;
    (size > 0).then_some(size).context("tmux input-buffer-size must be positive")
}

pub fn known_outer_terminal(pane: &str) -> Result<bool> {
    let identity =
        run_tmux(["display-message", "-p", "-t", pane, "#{client_termname},#{client_termtype}"])?;
    Ok(parse_client_terminal_identity(&identity))
}

#[must_use]
pub fn parse_client_terminal_identity(identity: &str) -> bool {
    let identity = identity.trim().to_ascii_lowercase();
    identity.contains("ghostty") || identity.contains("kitty")
}

#[cfg(test)]
mod tests {
    use super::{
        PaneGeometry, parse_client_terminal_identity, parse_input_buffer_size, parse_pane_geometry,
    };

    #[test]
    fn parses_pane_geometry() {
        assert_eq!(
            parse_pane_geometry("80,1,on,top\n").ok(),
            Some(PaneGeometry { column: 80, row: 2, status_top_rows: 1 })
        );
        assert_eq!(
            parse_pane_geometry("80,0,on,bottom\n").ok(),
            Some(PaneGeometry { column: 80, row: 0, status_top_rows: 0 })
        );
        assert!(parse_pane_geometry("80").is_err());
    }

    #[test]
    fn rejects_non_positive_input_buffer_sizes() {
        assert_eq!(parse_input_buffer_size("1048576\n").ok(), Some(1_048_576));
        assert!(parse_input_buffer_size("0").is_err());
    }

    #[test]
    fn recognizes_ghostty_and_kitty_client_identities() {
        assert!(parse_client_terminal_identity("xterm-ghostty,ghostty 1.4.0\n"));
        assert!(parse_client_terminal_identity("xterm-kitty,kitty"));
        assert!(!parse_client_terminal_identity("screen,xterm-256color"));
    }
}
