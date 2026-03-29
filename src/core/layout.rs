use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Viewport {
    pub width_px: u32,
    pub height_px: u32,
    pub scroll_y_px: u32,
}

impl Viewport {
    #[must_use]
    pub const fn new(width_px: u32, height_px: u32, scroll_y_px: u32) -> Self {
        Self { width_px, height_px, scroll_y_px }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LayoutIndex {
    pub y_offsets: Vec<u32>,
    pub total_height_px: u32,
}

impl LayoutIndex {
    #[must_use]
    pub fn new(y_offsets: Vec<u32>, total_height_px: u32) -> Self {
        Self { y_offsets, total_height_px }
    }
}

#[must_use]
pub fn visible_block_range(layout: &LayoutIndex, viewport: Viewport) -> Range<usize> {
    if layout.y_offsets.is_empty() {
        return 0..0;
    }

    let viewport_start = viewport.scroll_y_px;
    let viewport_end = viewport.scroll_y_px.saturating_add(viewport.height_px);

    let start = layout.y_offsets.iter().rposition(|offset| *offset <= viewport_start).unwrap_or(0);

    let end = layout
        .y_offsets
        .iter()
        .enumerate()
        .find_map(|(index, offset)| (*offset >= viewport_end).then_some(index))
        .unwrap_or(layout.y_offsets.len());

    start..end.max(start.saturating_add(1))
}
