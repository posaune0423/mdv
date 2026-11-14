use anyhow::{Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};

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
    Image { image_id: u32 },
}

#[must_use]
pub fn encode_transmit_png(image_id: u32, png_bytes: &[u8]) -> String {
    let payload = STANDARD.encode(png_bytes);
    const MAX_PAYLOAD_BYTES: usize = 4096;
    let mut chunks = payload.as_bytes().chunks(MAX_PAYLOAD_BYTES).peekable();
    let mut command = String::new();

    while let Some(chunk) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        let payload = std::str::from_utf8(chunk).unwrap_or_default();
        if command.is_empty() {
            command.push_str(&format!(
                "\u{1b}_Ga=t,t=d,f=100,i={image_id},q=2,m={more};{payload}\u{1b}\\"
            ));
        } else {
            command.push_str(&format!("\u{1b}_Gm={more},q=2;{payload}\u{1b}\\"));
        }
    }

    command
}

/// Encode all chunks for one PNG in a single tmux passthrough packet so tmux
/// cannot interleave continuation chunks from another pane.
pub fn encode_tmux_transmit_png(
    image_id: u32,
    png_bytes: &[u8],
    input_buffer_size: usize,
) -> Result<String> {
    encode_tmux_transfer(&encode_transmit_png(image_id, png_bytes), input_buffer_size)
}

pub fn encode_tmux_transfer(transfer: &str, input_buffer_size: usize) -> Result<String> {
    let wrapped = encode_tmux_passthrough(transfer);
    if wrapped.len() > input_buffer_size {
        bail!(
            "tmux input-buffer-size ({input_buffer_size}) is too small for this image transfer ({} bytes); increase it with: tmux set-option -s input-buffer-size {}",
            wrapped.len(),
            wrapped.len()
        );
    }
    Ok(wrapped)
}

#[must_use]
pub fn encode_place(placement: &KittyImagePlacement) -> String {
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

/// Keep normal cursor movement pane-local while placing the image at the
/// corresponding absolute outer-terminal position through tmux.
#[must_use]
pub fn encode_tmux_place_at(
    placement: &KittyImagePlacement,
    pane_column: u16,
    pane_row: u16,
    local_column: u16,
    local_row: u16,
) -> String {
    encode_tmux_command_at(&encode_place(placement), pane_column, pane_row, local_column, local_row)
}

#[must_use]
pub fn encode_tmux_command_at(
    kitty_command: &str,
    pane_column: u16,
    pane_row: u16,
    local_column: u16,
    local_row: u16,
) -> String {
    let local_cursor = ansi_cursor_move(local_column, local_row);
    let outer_cursor = ansi_cursor_move(
        pane_column.saturating_add(local_column),
        pane_row.saturating_add(local_row),
    );
    format!("{local_cursor}{}", encode_tmux_passthrough(&format!("{outer_cursor}{kitty_command}")))
}

#[must_use]
pub fn encode_tmux_delete(command: DeleteCommand) -> String {
    encode_tmux_passthrough(&encode_delete(command))
}

#[must_use]
pub fn encode_tmux_command(command: &str) -> String {
    encode_tmux_passthrough(command)
}

#[must_use]
pub fn encode_delete(command: DeleteCommand) -> String {
    match command {
        DeleteCommand::AllVisiblePlacements => "\u{1b}_Ga=d\u{1b}\\".to_string(),
        DeleteCommand::Placement { image_id, placement_id } => {
            format!("\u{1b}_Ga=d,d=i,i={image_id},p={placement_id}\u{1b}\\")
        }
        DeleteCommand::Image { image_id } => {
            format!("\u{1b}_Ga=d,d=I,i={image_id}\u{1b}\\")
        }
    }
}

fn ansi_cursor_move(column: u16, row: u16) -> String {
    format!("\u{1b}[{};{}H", row.saturating_add(1), column.saturating_add(1))
}

fn encode_tmux_passthrough(command: &str) -> String {
    format!("\u{1b}Ptmux;{}\u{1b}\\", command.replace('\u{1b}', "\u{1b}\u{1b}"))
}

const SUPPORT_QUERY_ID: u32 = 31;

/// Kitty graphics query followed by DA1, which terminals answer last.
#[must_use]
pub fn encode_support_query() -> String {
    format!("\u{1b}_Gi={SUPPORT_QUERY_ID},s=1,v=1,a=q,t=d,f=24;AAAA\u{1b}\\\u{1b}[c")
}

#[must_use]
pub fn encode_tmux_support_query() -> String {
    encode_tmux_passthrough(&encode_support_query())
}

/// Whether `OK` preceded the DA1 reply, or `None` until DA1 arrives.
#[must_use]
pub fn parse_support_reply(reply: &[u8]) -> Option<bool> {
    let da1_start = find_primary_device_attributes(reply)?;
    let before_da1 = String::from_utf8_lossy(reply.get(..da1_start)?);
    Some(before_da1.split("\u{1b}_G").skip(1).any(is_support_query_ok))
}

fn is_support_query_ok(apc: &str) -> bool {
    let Some((body, _)) = apc.split_once("\u{1b}\\") else {
        return false;
    };
    let id = format!("i={SUPPORT_QUERY_ID}");
    body.split_once(';')
        .is_some_and(|(keys, message)| message == "OK" && keys.split(',').any(|key| key == id))
}

fn find_primary_device_attributes(reply: &[u8]) -> Option<usize> {
    const PREFIX: &[u8] = b"\x1b[?";
    reply.windows(PREFIX.len()).enumerate().find_map(|(start, window)| {
        if window != PREFIX {
            return None;
        }
        let params = reply.get(start + PREFIX.len()..)?;
        let terminator = params.iter().find(|byte| !matches!(byte, b'0'..=b'9' | b';'))?;
        (*terminator == b'c').then_some(start)
    })
}
