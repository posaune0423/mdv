use std::io::Cursor;

use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use mdv::ui::page_graphics::{
    build_graphic_page, viewport_raster, viewport_slice, viewport_slice_at,
};

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
    assert_eq!(page.image.as_ref(), &ImageBuffer::from_pixel(4, 5, Rgba([255, 0, 0, 255])));
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
    assert!(decoded.to_rgba8().pixels().all(|pixel| *pixel == Rgba([0, 0, 255, 255])));
}

#[test]
fn graphic_page_rejects_a_truncated_snapshot_before_display() {
    let image = ImageBuffer::from_fn(64, 128, |x, y| Rgba([x as u8, y as u8, 40, 255]));
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture should encode: {error}"));
    png.truncate(png.len() / 2);
    assert!(
        build_graphic_page(&png, 32, 64).is_err(),
        "a partial page raster must fail before display"
    );
}

#[test]
fn viewport_rasters_preserve_pixels_at_top_middle_and_bottom() {
    let original = ImageBuffer::from_fn(16, 46, |x, y| {
        Rgba([x as u8, y as u8, 80, if x < 8 { 128 } else { 255 }])
    });
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(original.clone())
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture should encode: {error}"));
    let page = build_graphic_page(&png, 16, 8)
        .unwrap_or_else(|error| panic!("page should decode: {error}"));
    for scroll in [0, 1, 3] {
        let slice = viewport_slice(&page, scroll, 2, 5.5);
        let raster = viewport_raster(&page, scroll, 2, 5.5)
            .unwrap_or_else(|error| panic!("viewport should encode: {error}"))
            .unwrap_or_else(|| panic!("viewport should exist"));
        let actual = image::load_from_memory(&raster.png_bytes)
            .unwrap_or_else(|error| panic!("viewport should decode: {error}"))
            .to_rgba8();
        let expected =
            image::imageops::crop_imm(&original, 0, slice.source_y_px, 16, slice.source_height_px)
                .to_image();
        assert_eq!(
            actual, expected,
            "RGBA and orientation must survive viewport crop at row {scroll}"
        );
    }
}

#[test]
fn fractional_viewport_slice_uses_the_exact_rounded_raster_offset() {
    let image = ImageBuffer::from_fn(16, 64, |x, y| Rgba([x as u8, y as u8, 40, 255]));
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap_or_else(|error| panic!("fixture should encode: {error}"));
    let page = build_graphic_page(&png, 16, 8)
        .unwrap_or_else(|error| panic!("page should decode: {error}"));

    let fractional = viewport_slice_at(&page, 1.25, 3, 2.5);
    let settled = viewport_slice(&page, 1, 3, 2.5);

    assert_eq!(fractional.source_y_px, 6);
    assert_eq!(fractional.source_height_px, settled.source_height_px);
    assert_eq!(fractional.rows, settled.rows);
}

#[test]
fn associated_tiff_alpha_is_converted_to_straight_rgba() {
    let mut bytes = Vec::new();
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(Cursor::new(&mut bytes))
            .unwrap_or_else(|error| panic!("TIFF encoder: {error}"));
        let mut image = encoder
            .new_image::<tiff::encoder::colortype::RGBA8>(2, 1)
            .unwrap_or_else(|error| panic!("TIFF image: {error}"));
        image
            .encoder()
            .write_tag(tiff::tags::Tag::ExtraSamples, 1_u16)
            .unwrap_or_else(|error| panic!("associated alpha: {error}"));
        image
            .write_data(&[64, 32, 16, 128, 15, 128, 193, 254])
            .unwrap_or_else(|error| panic!("TIFF pixels: {error}"));
    }
    let page = build_graphic_page(&bytes, 2, 2)
        .unwrap_or_else(|error| panic!("TIFF should decode: {error}"));
    assert_eq!(*page.image.as_ref().get_pixel(0, 0), Rgba([128, 64, 32, 128]));
    assert_eq!(*page.image.as_ref().get_pixel(1, 0), Rgba([15, 129, 194, 254]));
}
