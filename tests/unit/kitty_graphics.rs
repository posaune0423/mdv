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
fn encodes_image_placement_command() {
    let placement = KittyImagePlacement {
        image_id: 7,
        placement_id: 3,
        columns: 12,
        rows: 4,
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
