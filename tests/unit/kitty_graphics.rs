use base64::Engine as _;
use mdv::io::kitty_graphics::{
    DeleteCommand, KittyImagePlacement, encode_delete, encode_place, encode_support_query,
    encode_tmux_support_query, encode_transmit_png, parse_support_reply,
};

#[test]
fn tmux_wraps_a_complete_transfer_in_one_dcs_packet() {
    let png_bytes = vec![42; 12_345];
    let direct = encode_transmit_png(7, &png_bytes);
    let transfer = mdv::io::kitty_graphics::encode_tmux_transmit_png(7, &png_bytes, 32_768)
        .unwrap_or_else(|error| panic!("tmux transfer should fit: {error}"));

    assert_eq!(transfer.matches("\u{1b}Ptmux;").count(), 1);
    assert_eq!(transfer.matches("\u{1b}_G").count(), 5);
    assert!(transfer.starts_with("\u{1b}Ptmux;"));
    assert!(transfer.ends_with("\u{1b}\\"));
    assert_eq!(
        transfer
            .trim_start_matches("\u{1b}Ptmux;")
            .strip_suffix("\u{1b}\\")
            .unwrap_or_else(|| panic!("tmux packet should end with DCS terminator"))
            .replace("\u{1b}\u{1b}", "\u{1b}"),
        direct,
        "tmux wrapping must not re-encode or alter the completed Kitty transfer"
    );
}

#[test]
fn tmux_rejects_a_transfer_larger_than_its_input_buffer() {
    let error = match mdv::io::kitty_graphics::encode_tmux_transmit_png(7, &[42; 64], 32) {
        Ok(_) => panic!("oversized tmux transfer should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("input-buffer-size"));
}

#[test]
fn tmux_placement_uses_the_outer_pane_origin_without_moving_tmux_cursor() {
    let placement = KittyImagePlacement {
        image_id: 31,
        placement_id: 31,
        columns: 10,
        rows: 4,
        source_x_px: 0,
        source_y_px: 0,
        source_width_px: 80,
        source_height_px: 64,
        cursor_x: 0,
        cursor_y: 0,
        z_index: -1,
    };
    let command = mdv::io::kitty_graphics::encode_tmux_place_at(&placement, 80, 1, 2, 3);

    assert!(command.starts_with("\u{1b}[4;3H\u{1b}Ptmux;"));
    assert!(command.contains("\u{1b}\u{1b}[5;83H"));
    assert!(command.contains("i=31,p=31"));
}

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

#[test]
fn support_reply_requires_ok_before_device_attributes() {
    assert_eq!(encode_support_query(), "\u{1b}_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\u{1b}\\\u{1b}[c");
    assert_eq!(parse_support_reply(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;22c"), Some(true));
    assert_eq!(parse_support_reply(b"\x1b[?62;22c"), Some(false));
    assert_eq!(parse_support_reply(b"\x1b[?62;22c\x1b_Gi=31;OK\x1b\\"), Some(false));
    assert_eq!(parse_support_reply(b"\x1b_Gi=31;ENOTSUPPORTED\x1b\\\x1b[?62;22c"), Some(false));
    assert_eq!(parse_support_reply(b"\x1b_Gi=7;OK\x1b\\\x1b[?62;22c"), Some(false));
    assert_eq!(parse_support_reply(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;2"), None);
}

#[test]
fn wraps_the_query_and_device_attributes_together_for_tmux() {
    let query = encode_tmux_support_query();
    assert_eq!(query.matches("\u{1b}Ptmux;").count(), 1);
    assert!(query.contains("\u{1b}\u{1b}_Gi=31"));
    assert!(query.contains("\u{1b}\u{1b}[c"));
}
