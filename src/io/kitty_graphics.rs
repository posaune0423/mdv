use std::process::Command;

use anyhow::{Context, Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};

const MAX_PAYLOAD_CHUNK_SIZE: usize = 4_096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KittyImagePlacement {
    pub image_id: u32,
    pub placement_id: u32,
    pub columns: u16,
    pub rows: u16,
    pub source_x_px: u32,
    pub source_y_px: u32,
    pub source_width_px: u32,
    pub source_height_px: u32,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub z_index: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeleteCommand {
    AllVisiblePlacements,
    Placement { image_id: u32, placement_id: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KittyGraphicsTransport {
    Direct,
    Tmux { pane_column: u16, pane_row: u16 },
}

impl KittyGraphicsTransport {
    #[must_use]
    pub const fn tmux_at(pane_column: u16, pane_row: u16) -> Self {
        Self::Tmux { pane_column, pane_row }
    }

    #[must_use]
    pub fn from_tmux_env(tmux: Option<&str>) -> Self {
        if tmux.is_some_and(|value| !value.is_empty()) { Self::tmux_at(0, 0) } else { Self::Direct }
    }

    fn encode(self, command: String) -> String {
        match self {
            Self::Direct => command,
            Self::Tmux { .. } => {
                let escaped = command.replace('\u{1b}', "\u{1b}\u{1b}");
                format!("\u{1b}Ptmux;{escaped}\u{1b}\\")
            }
        }
    }
}

pub fn tmux_pane_origin(pane_id: &str) -> Result<(u16, u16)> {
    let output = Command::new("tmux")
        .args(["display-message", "-p", "-t", pane_id, "#{pane_left},#{pane_top}"])
        .output()
        .context("failed to query tmux pane origin")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("tmux pane origin query failed: {}", stderr.trim());
    }

    let origin =
        String::from_utf8(output.stdout).context("tmux returned a non-UTF-8 pane origin")?;
    parse_tmux_pane_origin(&origin)
}

pub fn parse_tmux_pane_origin(origin: &str) -> Result<(u16, u16)> {
    let (column, row) =
        origin.trim().split_once(',').context("tmux pane origin must contain a column and row")?;
    Ok((
        column.parse().context("invalid tmux pane column")?,
        row.parse().context("invalid tmux pane row")?,
    ))
}

#[must_use]
pub fn encode_transmit_png(
    transport: KittyGraphicsTransport,
    image_id: u32,
    png_bytes: &[u8],
) -> String {
    let payload = STANDARD.encode(png_bytes);
    let chunk_count = payload.len().div_ceil(MAX_PAYLOAD_CHUNK_SIZE).max(1);
    let mut command = String::new();

    for index in 0..chunk_count {
        let start = index * MAX_PAYLOAD_CHUNK_SIZE;
        let end = payload.len().min(start + MAX_PAYLOAD_CHUNK_SIZE);
        let chunk = payload.get(start..end).unwrap_or_default();
        let more = u8::from(index + 1 < chunk_count);

        let chunk_command = if index == 0 {
            format!("\u{1b}_Ga=t,t=d,f=100,i={image_id},q=2,m={more};{chunk}\u{1b}\\")
        } else {
            format!("\u{1b}_Gm={more},q=2;{chunk}\u{1b}\\")
        };
        command.push_str(&transport.encode(chunk_command));
    }

    command
}

#[must_use]
pub fn encode_place(transport: KittyGraphicsTransport, placement: &KittyImagePlacement) -> String {
    transport.encode(placement_command(placement))
}

#[must_use]
pub fn encode_place_at(
    transport: KittyGraphicsTransport,
    placement: &KittyImagePlacement,
    column: u16,
    row: u16,
) -> String {
    let local_cursor = ansi_cursor_move(column, row);
    match transport {
        KittyGraphicsTransport::Direct => {
            format!("{local_cursor}{}", placement_command(placement))
        }
        KittyGraphicsTransport::Tmux { pane_column, pane_row } => {
            let outer_cursor =
                ansi_cursor_move(pane_column.saturating_add(column), pane_row.saturating_add(row));
            format!(
                "{local_cursor}{}",
                transport.encode(format!("{outer_cursor}{}", placement_command(placement)))
            )
        }
    }
}

fn placement_command(placement: &KittyImagePlacement) -> String {
    format!(
        "\u{1b}_Ga=p,i={},p={},c={},r={},x={},y={},w={},h={},X={},Y={},z={},q=2,C=1\u{1b}\\",
        placement.image_id,
        placement.placement_id,
        placement.columns,
        placement.rows,
        placement.source_x_px,
        placement.source_y_px,
        placement.source_width_px,
        placement.source_height_px,
        placement.cursor_x,
        placement.cursor_y,
        placement.z_index,
    )
}

fn ansi_cursor_move(column: u16, row: u16) -> String {
    format!("\u{1b}[{};{}H", row.saturating_add(1), column.saturating_add(1))
}

#[must_use]
pub fn encode_delete(transport: KittyGraphicsTransport, command: DeleteCommand) -> String {
    transport.encode(match command {
        DeleteCommand::AllVisiblePlacements => "\u{1b}_Ga=d\u{1b}\\".to_string(),
        DeleteCommand::Placement { image_id, placement_id } => {
            format!("\u{1b}_Ga=d,d=i,i={image_id},p={placement_id}\u{1b}\\")
        }
    })
}
