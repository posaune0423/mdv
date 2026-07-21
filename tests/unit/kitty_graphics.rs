use mdv::io::kitty_graphics::{
    DeleteCommand, KittyGraphicsTransport, KittyImagePlacement, encode_delete, encode_place,
    encode_transmit_png, parse_tmux_pane_origin,
};

#[test]
fn encodes_png_transfer_command() {
    let escape = encode_transmit_png(KittyGraphicsTransport::Direct, 7, &[1, 2, 3, 4]);

    assert!(escape.contains("a=t"));
    assert!(escape.contains("f=100"));
    assert!(escape.contains("i=7"));
}

#[test]
fn chunks_png_payloads_at_4096_base64_bytes() {
    let escape = encode_transmit_png(KittyGraphicsTransport::Direct, 7, &vec![0; 3_073]);
    let chunks = escape.split("\u{1b}\\").filter(|chunk| !chunk.is_empty()).collect::<Vec<_>>();

    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].starts_with("\u{1b}_Ga=t,t=d,f=100,i=7,q=2,m=1;"));
    assert!(chunks[1].starts_with("\u{1b}_Gm=0,q=2;"));
    assert!(
        chunks.iter().all(|chunk| {
            chunk.split_once(';').is_some_and(|(_, payload)| payload.len() <= 4_096)
        })
    );
}

#[test]
fn wraps_each_png_chunk_in_its_own_tmux_passthrough() {
    let escape = encode_transmit_png(KittyGraphicsTransport::tmux_at(0, 0), 7, &vec![0; 3_073]);

    assert_eq!(escape.matches("\u{1b}Ptmux;").count(), 2);
    assert!(escape.contains("_Ga=t,t=d,f=100,i=7,q=2,m=1;"));
    assert!(escape.contains("_Gm=0,q=2;"));
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

    let escape = encode_place(KittyGraphicsTransport::Direct, &placement);

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
    let escape = encode_delete(KittyGraphicsTransport::Direct, DeleteCommand::AllVisiblePlacements);
    assert_eq!(escape, "\u{1b}_Ga=d\u{1b}\\");
}

#[test]
fn encodes_delete_specific_placement_command() {
    let escape = encode_delete(
        KittyGraphicsTransport::Direct,
        DeleteCommand::Placement { image_id: 7, placement_id: 3 },
    );
    assert_eq!(escape, "\u{1b}_Ga=d,d=i,i=7,p=3\u{1b}\\");
}

#[test]
fn wraps_kitty_commands_in_tmux_dcs_passthrough() {
    let escape =
        encode_delete(KittyGraphicsTransport::tmux_at(0, 0), DeleteCommand::AllVisiblePlacements);

    assert_eq!(escape, "\u{1b}Ptmux;\u{1b}\u{1b}_Ga=d\u{1b}\u{1b}\\\u{1b}\\");
}

#[test]
fn detects_tmux_transport_from_environment_value() {
    assert_eq!(
        KittyGraphicsTransport::from_tmux_env(Some("/private/tmp/tmux/default,1,0")),
        KittyGraphicsTransport::tmux_at(0, 0),
    );
    assert_eq!(KittyGraphicsTransport::from_tmux_env(None), KittyGraphicsTransport::Direct);
    assert_eq!(KittyGraphicsTransport::from_tmux_env(Some("")), KittyGraphicsTransport::Direct);
}

#[test]
fn parses_tmux_pane_origin_coordinates() {
    assert_eq!(parse_tmux_pane_origin("128,3\n").ok(), Some((128, 3)));
    assert!(parse_tmux_pane_origin("128").is_err());
}
