use base64::Engine as _;
use mdv::io::kitty_graphics::{
    DeleteCommand, KittyImagePlacement, encode_delete, encode_place, encode_transmit_png,
};

#[test]
fn encodes_png_transfer_command() {
    let escape = encode_transmit_png(7, &[1, 2, 3, 4]);

    assert!(escape.contains("a=t"));
    assert!(escape.contains("f=100"));
    assert!(escape.contains("i=7"));
}

#[test]
fn chunks_large_png_transfers_without_changing_the_payload() {
    let png_bytes = (0..12_345).map(|index| (index % 251) as u8).collect::<Vec<_>>();

    let escape = encode_transmit_png(7, &png_bytes);
    let chunks = escape.split("\u{1b}\\").filter(|chunk| !chunk.is_empty()).collect::<Vec<_>>();

    assert!(chunks.len() > 1);
    assert!(chunks.first().is_some_and(|chunk| chunk.contains("a=t")));
    assert!(chunks.first().is_some_and(|chunk| chunk.contains("m=1")));
    assert!(chunks.last().is_some_and(|chunk| chunk.contains("m=0")));

    let encoded = chunks
        .iter()
        .map(|chunk| chunk.split_once(';').map_or("", |(_, payload)| payload))
        .collect::<String>();
    assert!(
        chunks.iter().all(|chunk| {
            chunk.split_once(';').is_some_and(|(_, payload)| payload.len() <= 4096)
        })
    );
    let reassembled = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap_or_else(|error| panic!("chunked payload should decode: {error}"));
    assert_eq!(reassembled, png_bytes);
}

#[test]
fn encodes_image_placement_command() {
    let placement = KittyImagePlacement {
        image_id: 7,
        placement_id: 3,
        columns: 12,
        rows: 4,
        source_x_px: 11,
        source_y_px: 13,
        source_width_px: 101,
        source_height_px: 79,
        cursor_x: 2,
        cursor_y: 5,
        z_index: -1,
    };

    let escape = encode_place(&placement);

    assert!(escape.contains("a=p"));
    assert!(escape.contains("i=7"));
    assert!(escape.contains("p=3"));
    assert!(escape.contains("c=12"));
    assert!(escape.contains("r=4"));
    assert!(escape.contains("x=11"));
    assert!(escape.contains("y=13"));
    assert!(escape.contains("w=101"));
    assert!(escape.contains("h=79"));
    assert!(escape.contains("X=2"));
    assert!(escape.contains("Y=5"));
}

#[test]
fn encodes_delete_visible_placements_command() {
    let escape = encode_delete(DeleteCommand::AllVisiblePlacements);
    assert_eq!(escape, "\u{1b}_Ga=d\u{1b}\\");
}

#[test]
fn encodes_delete_specific_placement_command() {
    let escape = encode_delete(DeleteCommand::Placement { image_id: 7, placement_id: 3 });
    assert_eq!(escape, "\u{1b}_Ga=d,d=i,i=7,p=3\u{1b}\\");
}

#[test]
fn encodes_delete_image_data_command() {
    let escape = encode_delete(DeleteCommand::Image { image_id: 7 });
    assert_eq!(escape, "\u{1b}_Ga=d,d=I,i=7\u{1b}\\");
}
