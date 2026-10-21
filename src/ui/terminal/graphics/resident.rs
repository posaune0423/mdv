use std::{
    io::Cursor,
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
    thread,
};

use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat, RgbaImage};

use crate::{
    io::kitty_graphics::{
        DeleteCommand, KittyImagePlacement, encode_delete, encode_place, encode_transmit_png,
    },
    ui::{
        page_graphics::{GraphicPage, ViewportSlice, viewport_slice_at},
        terminal::CellMetrics,
    },
};

use super::ansi_cursor_move;

const RESIDENT_IMAGE_IDS: [u32; 2] = [1, u32::MAX];
const MAX_RESIDENT_WINDOWS: usize = RESIDENT_IMAGE_IDS.len();
const WINDOW_VIEWPORT_MULTIPLIER: u32 = 3;
const MAX_WINDOW_HEIGHT_PX: u32 = 4096;

type GraphicCommands = (Vec<String>, Vec<(u32, u32)>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PageKey {
    image_ptr: usize,
    image_width_px: u32,
    image_height_px: u32,
    display_width_px: u32,
    display_height_px: u32,
}

impl PageKey {
    fn of(page: &GraphicPage) -> Self {
        Self {
            image_ptr: page.image.as_ref().as_raw().as_ptr() as usize,
            image_width_px: page.image_width_px,
            image_height_px: page.image_height_px,
            display_width_px: page.display_width_px,
            display_height_px: page.display_height_px,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WindowSpec {
    page: PageKey,
    source_y_px: u32,
    source_height_px: u32,
}

#[derive(Clone, Copy, Debug)]
struct ResidentWindow {
    image_id: u32,
    source_y_px: u32,
    source_height_px: u32,
}

impl ResidentWindow {
    fn contains(self, slice: ViewportSlice) -> bool {
        let source_end = slice.source_y_px.saturating_add(slice.source_height_px);
        let window_end = self.source_y_px.saturating_add(self.source_height_px);
        slice.source_y_px >= self.source_y_px && source_end <= window_end
    }

    fn matches(self, spec: WindowSpec) -> bool {
        self.source_y_px == spec.source_y_px && self.source_height_px == spec.source_height_px
    }

    fn distance_from(self, source_y_px: u32) -> u32 {
        let window_end = self.source_y_px.saturating_add(self.source_height_px);
        if source_y_px < self.source_y_px {
            self.source_y_px.saturating_sub(source_y_px)
        } else {
            source_y_px.saturating_sub(window_end)
        }
    }
}

struct PreparedWindow {
    spec: WindowSpec,
    png_bytes: Vec<u8>,
}

struct PendingPreparation {
    generation: u64,
    receiver: Receiver<Result<PreparedWindow>>,
}

pub(in crate::ui::terminal) struct PageGraphics {
    page: Option<PageKey>,
    windows: Vec<ResidentWindow>,
    active_image_id: Option<u32>,
    last_source_y_px: Option<u32>,
    motion_direction: i8,
    generation: u64,
    pending: Option<PendingPreparation>,
}

impl Default for PageGraphics {
    fn default() -> Self {
        Self {
            page: None,
            windows: Vec::new(),
            active_image_id: None,
            last_source_y_px: None,
            motion_direction: 1,
            generation: 0,
            pending: None,
        }
    }
}

impl PageGraphics {
    pub(in crate::ui::terminal) fn commands(
        &mut self,
        page: &GraphicPage,
        scroll_rows: f64,
        content_height: usize,
        cell_metrics: CellMetrics,
        previous_visible_placements: &[(u32, u32)],
    ) -> Result<GraphicCommands> {
        let mut commands = self.invalidate_for_page(page);
        let slice = viewport_slice_at(page, scroll_rows, content_height, cell_metrics.height_px);
        if slice.rows == 0 || slice.source_height_px == 0 {
            self.active_image_id = None;
            commands.extend(delete_hidden_placements(previous_visible_placements, &[]));
            return Ok((commands, Vec::new()));
        }

        let initial_upload = self.windows.is_empty();
        if let Some(index) = self.containing_window_index(slice) {
            self.active_image_id = Some(self.windows[index].image_id);
        }
        commands.extend(self.maintenance()?);
        self.update_motion(slice.source_y_px);
        let window_index = match self.containing_window_index(slice) {
            Some(index) => index,
            None => self.upload_window(page, slice, initial_upload, &mut commands)?,
        };
        let window = self.windows[window_index];
        let image_id = window.image_id;
        self.active_image_id = Some(image_id);
        if initial_upload {
            self.schedule_initial_preparation(page, slice)?;
        } else {
            self.schedule_lookahead(page, slice, window)?;
        }

        let visible_placements = vec![(image_id, image_id)];
        commands.extend(delete_hidden_placements(previous_visible_placements, &visible_placements));
        let placement = KittyImagePlacement {
            image_id,
            placement_id: image_id,
            columns: page.width_cells,
            rows: slice.rows,
            source_x_px: 0,
            source_y_px: slice.source_y_px.saturating_sub(window.source_y_px),
            source_width_px: page.image_width_px,
            source_height_px: slice.source_height_px,
            cursor_x: 0,
            cursor_y: 0,
            z_index: -1,
        };
        commands.push(format!("{}{}", ansi_cursor_move(0, 0), encode_place(&placement)));

        Ok((commands, visible_placements))
    }

    pub(in crate::ui::terminal) fn has_pending_work(&self) -> bool {
        self.pending.is_some()
    }

    pub(in crate::ui::terminal) fn maintenance(&mut self) -> Result<Vec<String>> {
        let Some(pending) = &self.pending else {
            return Ok(Vec::new());
        };
        match pending.receiver.try_recv() {
            Ok(result) => {
                let generation = pending.generation;
                self.pending = None;
                self.apply_prepared(generation, result?)
            }
            Err(TryRecvError::Empty) => Ok(Vec::new()),
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                Ok(Vec::new())
            }
        }
    }

    pub(in crate::ui::terminal) fn reset(&mut self) -> Vec<String> {
        self.generation = self.generation.wrapping_add(1);
        self.page = None;
        self.pending = None;
        self.active_image_id = None;
        self.last_source_y_px = None;
        self.motion_direction = 1;
        self.windows
            .drain(..)
            .map(|window| encode_delete(DeleteCommand::Image { image_id: window.image_id }))
            .collect()
    }

    fn invalidate_for_page(&mut self, page: &GraphicPage) -> Vec<String> {
        let key = PageKey::of(page);
        if self.page == Some(key) {
            return Vec::new();
        }

        let deletes = self.reset();
        self.page = Some(key);
        deletes
    }

    fn update_motion(&mut self, source_y_px: u32) {
        if let Some(previous) = self.last_source_y_px {
            self.motion_direction = if source_y_px >= previous { 1 } else { -1 };
        }
        self.last_source_y_px = Some(source_y_px);
    }

    fn containing_window_index(&self, slice: ViewportSlice) -> Option<usize> {
        self.active_image_id
            .and_then(|image_id| {
                self.windows
                    .iter()
                    .position(|window| window.image_id == image_id && window.contains(slice))
            })
            .or_else(|| self.windows.iter().position(|window| window.contains(slice)))
    }

    fn schedule_initial_preparation(
        &mut self,
        page: &GraphicPage,
        slice: ViewportSlice,
    ) -> Result<()> {
        self.schedule_spec(page, resident_spec(page, slice))
    }

    fn schedule_lookahead(
        &mut self,
        page: &GraphicPage,
        slice: ViewportSlice,
        active: ResidentWindow,
    ) -> Result<()> {
        if self.pending.is_some() {
            return Ok(());
        }
        let Some(target_slice) = adjacent_slice(page, slice, active, self.motion_direction) else {
            return Ok(());
        };
        self.schedule_spec(page, resident_spec(page, target_slice))
    }

    fn schedule_spec(&mut self, page: &GraphicPage, spec: WindowSpec) -> Result<()> {
        if self.pending.is_some() {
            return Ok(());
        }
        if self.windows.iter().any(|window| window.matches(spec)) {
            return Ok(());
        }

        let image = Arc::clone(&page.image);
        let width_px = page.image_width_px;
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("mdv-page-graphics".to_string())
            .spawn(move || {
                let result = encode_window(image, width_px, spec);
                let _ = sender.send(result);
            })
            .context("failed to start resident graphics preparation")?;
        self.pending = Some(PendingPreparation { generation: self.generation, receiver });
        Ok(())
    }

    fn apply_prepared(&mut self, generation: u64, prepared: PreparedWindow) -> Result<Vec<String>> {
        if generation != self.generation
            || self.page != Some(prepared.spec.page)
            || self.windows.iter().any(|window| window.matches(prepared.spec))
        {
            return Ok(Vec::new());
        }

        let (slot, image_id) = self.slot_for_window(prepared.spec.source_y_px);
        let mut commands = Vec::new();
        if slot < self.windows.len() {
            commands.push(encode_delete(DeleteCommand::Image { image_id }));
            self.windows[slot] = ResidentWindow {
                image_id,
                source_y_px: prepared.spec.source_y_px,
                source_height_px: prepared.spec.source_height_px,
            };
        } else {
            self.windows.push(ResidentWindow {
                image_id,
                source_y_px: prepared.spec.source_y_px,
                source_height_px: prepared.spec.source_height_px,
            });
        }
        commands.push(encode_transmit_png(image_id, &prepared.png_bytes));
        Ok(commands)
    }

    fn upload_window(
        &mut self,
        page: &GraphicPage,
        slice: ViewportSlice,
        exact_viewport: bool,
        commands: &mut Vec<String>,
    ) -> Result<usize> {
        let spec = if exact_viewport {
            WindowSpec {
                page: PageKey::of(page),
                source_y_px: slice.source_y_px,
                source_height_px: slice.source_height_px,
            }
        } else {
            resident_spec(page, slice)
        };
        let prepared = encode_window(Arc::clone(&page.image), page.image_width_px, spec)?;
        let (slot, image_id) = self.slot_for_window(slice.source_y_px);
        if slot < self.windows.len() {
            commands.push(encode_delete(DeleteCommand::Image { image_id }));
            self.windows[slot] = ResidentWindow {
                image_id,
                source_y_px: spec.source_y_px,
                source_height_px: spec.source_height_px,
            };
        } else {
            self.windows.push(ResidentWindow {
                image_id,
                source_y_px: spec.source_y_px,
                source_height_px: spec.source_height_px,
            });
        }
        commands.push(encode_transmit_png(image_id, &prepared.png_bytes));
        Ok(slot)
    }

    fn slot_for_window(&self, source_y_px: u32) -> (usize, u32) {
        if self.windows.len() < MAX_RESIDENT_WINDOWS {
            let slot = self.windows.len();
            return (slot, RESIDENT_IMAGE_IDS[slot]);
        }
        let slot = self
            .windows
            .iter()
            .enumerate()
            .filter(|(_, window)| Some(window.image_id) != self.active_image_id)
            .max_by_key(|(_, window)| window.distance_from(source_y_px))
            .map_or(0, |(index, _)| index);
        (slot, self.windows[slot].image_id)
    }
}

fn encode_window(image: Arc<RgbaImage>, width_px: u32, spec: WindowSpec) -> Result<PreparedWindow> {
    let cropped = image::imageops::crop_imm(
        image.as_ref(),
        0,
        spec.source_y_px,
        width_px,
        spec.source_height_px,
    )
    .to_image();
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(cropped)
        .write_to(&mut png, ImageFormat::Png)
        .context("failed to encode resident page window")?;
    Ok(PreparedWindow { spec, png_bytes: png.into_inner() })
}

fn resident_height(page: &GraphicPage, viewport_height_px: u32) -> u32 {
    let desired = viewport_height_px
        .saturating_mul(WINDOW_VIEWPORT_MULTIPLIER)
        .min(MAX_WINDOW_HEIGHT_PX)
        .max(viewport_height_px);
    desired.min(page.image_height_px)
}

fn resident_spec(page: &GraphicPage, slice: ViewportSlice) -> WindowSpec {
    let source_height_px = resident_height(page, slice.source_height_px);
    WindowSpec {
        page: PageKey::of(page),
        source_y_px: resident_start(page, slice, source_height_px),
        source_height_px,
    }
}

fn resident_start(page: &GraphicPage, slice: ViewportSlice, resident_height_px: u32) -> u32 {
    let margin = resident_height_px.saturating_sub(slice.source_height_px) / 2;
    slice
        .source_y_px
        .saturating_sub(margin)
        .min(page.image_height_px.saturating_sub(resident_height_px))
}

fn adjacent_slice(
    page: &GraphicPage,
    slice: ViewportSlice,
    active: ResidentWindow,
    direction: i8,
) -> Option<ViewportSlice> {
    let maximum_start = page.image_height_px.saturating_sub(slice.source_height_px);
    let source_y_px = if direction < 0 {
        active.source_y_px.saturating_sub(slice.source_height_px)
    } else {
        active
            .source_y_px
            .saturating_add(active.source_height_px)
            .saturating_sub(slice.source_height_px)
            .min(maximum_start)
    };
    if source_y_px == slice.source_y_px && active.contains(slice) {
        return None;
    }
    let source_height_px =
        slice.source_height_px.min(page.image_height_px.saturating_sub(source_y_px));
    (source_height_px > 0).then_some(ViewportSlice {
        source_y_px,
        source_height_px,
        rows: slice.rows,
    })
}

fn delete_hidden_placements(
    previous_visible_placements: &[(u32, u32)],
    visible_placements: &[(u32, u32)],
) -> Vec<String> {
    previous_visible_placements
        .iter()
        .copied()
        .filter(|placement| !visible_placements.contains(placement))
        .map(|(image_id, placement_id)| {
            encode_delete(DeleteCommand::Placement { image_id, placement_id })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::mpsc};

    use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};

    use crate::ui::page_graphics::build_graphic_page;

    use super::*;

    fn page() -> GraphicPage {
        let image = ImageBuffer::from_fn(8, 160, |x, y| Rgba([x as u8, y as u8, 40, 255]));
        let mut png = Vec::new();
        DynamicImage::ImageRgba8(image)
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap_or_else(|error| panic!("fixture should encode: {error}"));
        build_graphic_page(&png, 16, 8)
            .unwrap_or_else(|error| panic!("fixture page should decode: {error}"))
    }

    fn wait_until_ready_without_applying(graphics: &mut PageGraphics) {
        let pending = graphics
            .pending
            .take()
            .unwrap_or_else(|| panic!("a resident preparation should be pending"));
        let prepared = pending
            .receiver
            .recv()
            .unwrap_or_else(|error| panic!("resident preparation should complete: {error}"));
        let (sender, receiver) = mpsc::channel();
        sender
            .send(prepared)
            .unwrap_or_else(|error| panic!("prepared result should remain available: {error}"));
        graphics.pending = Some(PendingPreparation { generation: pending.generation, receiver });
    }

    #[test]
    fn ready_forward_preparation_keeps_the_reversed_resident_window() {
        let page = page();
        let metrics = CellMetrics { width_px: 8.0, height_px: 2.0 };
        let mut graphics = PageGraphics::default();

        let (_, first_visible) = graphics
            .commands(&page, 0.0, 4, metrics, &[])
            .unwrap_or_else(|error| panic!("first window should render: {error}"));
        wait_until_ready_without_applying(&mut graphics);
        let _ = graphics
            .maintenance()
            .unwrap_or_else(|error| panic!("forward window should prepare: {error}"));
        let (_, forward_visible) = graphics
            .commands(&page, 13.0, 4, metrics, &first_visible)
            .unwrap_or_else(|error| panic!("forward resident should render: {error}"));
        wait_until_ready_without_applying(&mut graphics);

        let (reversed, _) = graphics
            .commands(&page, 0.0, 4, metrics, &forward_visible)
            .unwrap_or_else(|error| panic!("reversed resident should render: {error}"));

        assert_eq!(reversed.iter().filter(|command| command.contains("a=t")).count(), 1);
        assert!(reversed.iter().any(|command| command.contains("a=p,i=1")));
    }
}
