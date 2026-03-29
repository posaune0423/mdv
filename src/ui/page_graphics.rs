use std::io::Cursor;

use anyhow::{Context, Result};
use image::{DynamicImage, ImageDecoder, ImageFormat, codecs::png::PngDecoder};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphicPage {
    pub width_cells: u16,
    pub display_width_px: u32,
    pub display_height_px: u32,
    pub image_width_px: u32,
    pub image_height_px: u32,
    pub png_bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewportSlice {
    pub source_y_px: u32,
    pub source_height_px: u32,
    pub rows: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewportRaster {
    pub png_bytes: Vec<u8>,
    pub image_width_px: u32,
    pub image_height_px: u32,
    pub rows: u16,
}

pub fn build_graphic_page(
    png_bytes: &[u8],
    width_cells: u16,
    display_width_px: u32,
) -> Result<GraphicPage> {
    let decoder = PngDecoder::new(Cursor::new(png_bytes))
        .context("failed to decode page snapshot png header")?;
    let (image_width_px, image_height_px) = decoder.dimensions();
    let display_width_px = display_width_px.max(1);
    let raster_scale = image_width_px as f32 / display_width_px as f32;
    let display_height_px = ((image_height_px as f32) / raster_scale).round().max(1.0) as u32;

    Ok(GraphicPage {
        width_cells,
        display_width_px,
        display_height_px,
        image_width_px,
        image_height_px,
        png_bytes: png_bytes.to_vec(),
    })
}

#[must_use]
pub fn viewport_slice(
    page: &GraphicPage,
    scroll_rows: usize,
    content_rows: usize,
    cell_height_px: f32,
) -> ViewportSlice {
    let total_rows = total_rows(page, cell_height_px);
    let rows = total_rows.saturating_sub(scroll_rows).min(content_rows) as u16;
    if rows == 0 {
        return ViewportSlice { source_y_px: 0, source_height_px: 0, rows: 0 };
    }

    let raster_scale = page.image_width_px as f32 / page.display_width_px.max(1) as f32;
    let top_display_px = ((scroll_rows as f32) * cell_height_px).round().max(0.0);
    let target_display_height_px = (((rows as f32) * cell_height_px).round())
        .min(page.display_height_px.saturating_sub(top_display_px as u32) as f32)
        .max(1.0);
    let source_y_px = (top_display_px * raster_scale).round().max(0.0) as u32;
    let unclamped_source_height_px =
        (target_display_height_px * raster_scale).round().max(1.0) as u32;
    let source_height_px =
        unclamped_source_height_px.min(page.image_height_px.saturating_sub(source_y_px));

    ViewportSlice { source_y_px, source_height_px, rows }
}

pub fn viewport_raster(
    page: &GraphicPage,
    scroll_rows: usize,
    content_rows: usize,
    cell_height_px: f32,
) -> Result<Option<ViewportRaster>> {
    let slice = viewport_slice(page, scroll_rows, content_rows, cell_height_px);
    if slice.rows == 0 || slice.source_height_px == 0 {
        return Ok(None);
    }

    let image = image::load_from_memory_with_format(&page.png_bytes, ImageFormat::Png)
        .context("failed to decode page snapshot png for viewport crop")?;
    let cropped = image.crop_imm(0, slice.source_y_px, page.image_width_px, slice.source_height_px);
    let mut png_bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(cropped.to_rgba8())
        .write_to(&mut png_bytes, ImageFormat::Png)
        .context("failed to encode cropped viewport png")?;

    Ok(Some(ViewportRaster {
        png_bytes: png_bytes.into_inner(),
        image_width_px: page.image_width_px,
        image_height_px: slice.source_height_px,
        rows: slice.rows,
    }))
}

#[must_use]
pub fn total_rows(page: &GraphicPage, cell_height_px: f32) -> usize {
    ((page.display_height_px as f32) / cell_height_px).ceil().max(0.0) as usize
}
