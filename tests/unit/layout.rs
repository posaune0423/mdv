use mdv::core::layout::{LayoutIndex, Viewport, visible_block_range};

#[test]
fn visible_range_returns_blocks_intersecting_viewport() {
    let layout = LayoutIndex::new(vec![0, 40, 120, 240], 320);
    let viewport = Viewport::new(800, 100, 60);

    let range = visible_block_range(&layout, viewport);

    assert_eq!(range, 1..3);
}
