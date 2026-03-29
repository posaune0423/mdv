use std::io::Cursor;

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use mdv::ui::page_graphics::{build_graphic_page, viewport_raster, viewport_slice};

#[test]
fn builds_graphic_page_from_full_snapshot_png() {
    let image = ImageBuffer::from_pixel(4, 5, Rgba([255, 0, 0, 255]));
    let mut png_bytes = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture image should encode: {error}"));

    let page = build_graphic_page(&png_bytes, 12, 4)
        .unwrap_or_else(|error| panic!("graphic page should decode: {error}"));

    assert_eq!(page.width_cells, 12);
    assert_eq!(page.display_width_px, 4);
    assert_eq!(page.display_height_px, 5);
    assert_eq!(page.image_width_px, 4);
    assert_eq!(page.image_height_px, 5);
    assert_eq!(page.png_bytes, png_bytes);
}

#[test]
fn viewport_slice_accounts_for_raster_scale_without_distorting_terminal_rows() {
    let image = ImageBuffer::from_pixel(16, 46, Rgba([255, 0, 0, 255]));
    let mut png_bytes = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture image should encode: {error}"));

    let page = build_graphic_page(&png_bytes, 16, 8)
        .unwrap_or_else(|error| panic!("graphic page should decode: {error}"));
    let slice = viewport_slice(&page, 1, 4, 5.5);

    assert_eq!(page.display_height_px, 23);
    assert_eq!(slice.source_y_px, 12);
    assert_eq!(slice.source_height_px, 34);
    assert_eq!(slice.rows, 4);
}

#[test]
fn viewport_slice_clamps_bottom_without_stretching_extra_blank_rows() {
    let image = ImageBuffer::from_pixel(8, 10, Rgba([255, 0, 0, 255]));
    let mut png_bytes = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture image should encode: {error}"));

    let page = build_graphic_page(&png_bytes, 16, 8)
        .unwrap_or_else(|error| panic!("graphic page should decode: {error}"));
    let slice = viewport_slice(&page, 3, 4, 2.0);

    assert_eq!(slice.source_y_px, 6);
    assert_eq!(slice.source_height_px, 4);
    assert_eq!(slice.rows, 2);
}

#[test]
fn viewport_raster_encodes_only_the_visible_crop() {
    let image = ImageBuffer::from_fn(8, 10, |_, y| {
        if y < 6 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) }
    });
    let mut png_bytes = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture image should encode: {error}"));

    let page = build_graphic_page(&png_bytes, 16, 8)
        .unwrap_or_else(|error| panic!("graphic page should decode: {error}"));
    let raster = viewport_raster(&page, 3, 4, 2.0)
        .unwrap_or_else(|error| panic!("viewport raster should encode: {error}"))
        .unwrap_or_else(|| panic!("viewport raster should exist"));

    let decoded = image::load_from_memory_with_format(&raster.png_bytes, ImageFormat::Png)
        .unwrap_or_else(|error| panic!("cropped viewport png should decode: {error}"));

    assert_eq!(raster.rows, 2);
    assert_eq!(raster.image_width_px, 8);
    assert_eq!(raster.image_height_px, 4);
    assert_eq!(decoded.width(), 8);
    assert_eq!(decoded.height(), 4);
}
