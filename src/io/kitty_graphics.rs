use base64::{Engine as _, engine::general_purpose::STANDARD};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KittyImagePlacement {
    pub image_id: u32,
    pub placement_id: u32,
    pub columns: u16,
    pub rows: u16,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub z_index: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeleteCommand {
    AllVisiblePlacements,
    Placement { image_id: u32, placement_id: u32 },
}

#[must_use]
pub fn encode_transmit_png(image_id: u32, png_bytes: &[u8]) -> String {
    let payload = STANDARD.encode(png_bytes);
    format!(
        "\u{1b}_Ga=t,t=d,f=100,i={},q=2;{}\u{1b}\\",
        image_id,
        payload
    )
}

#[must_use]
pub fn encode_place(placement: &KittyImagePlacement) -> String {
    format!(
        "\u{1b}_Ga=p,i={},p={},c={},r={},X={},Y={},z={},q=2,C=1\u{1b}\\",
        placement.image_id,
        placement.placement_id,
        placement.columns,
        placement.rows,
        placement.cursor_x,
        placement.cursor_y,
        placement.z_index,
    )
}

#[must_use]
pub fn encode_delete(command: DeleteCommand) -> String {
    match command {
        DeleteCommand::AllVisiblePlacements => "\u{1b}_Ga=d\u{1b}\\".to_string(),
        DeleteCommand::Placement { image_id, placement_id } => {
            format!("\u{1b}_Ga=d,d=i,i={image_id},p={placement_id}\u{1b}\\")
        }
    }
}
