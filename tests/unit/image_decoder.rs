use image::{ImageBuffer, Rgba};
use mdv::io::image_decoder::ImageDecoder;
use tempfile::TempDir;

#[test]
fn decodes_png_dimensions_and_bytes() {
    let dir = match TempDir::new() {
        Ok(dir) => dir,
        Err(error) => panic!("temp dir should be created: {error}"),
    };
    let file = dir.path().join("pixel.png");
    let image = ImageBuffer::from_pixel(1, 1, Rgba([255_u8, 0_u8, 0_u8, 255_u8]));
    if let Err(error) = image.save(&file) {
        panic!("fixture should be written: {error}");
    }

    let decoder = ImageDecoder::new();
    let loaded = match decoder.load_png(&file) {
        Ok(loaded) => loaded,
        Err(error) => panic!("image should decode: {error}"),
    };

    assert_eq!(loaded.width, 1);
    assert_eq!(loaded.height, 1);
    assert!(!loaded.png_bytes.is_empty());
}
