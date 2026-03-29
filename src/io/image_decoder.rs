use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::Result;
use image::{GenericImageView, ImageFormat, ImageReader};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedImage {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub png_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct ImageDecoder;

impl ImageDecoder {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub fn load_from_document(&self, document_path: &Path, src: &str) -> Result<LoadedImage> {
        let path = resolve_image_path(document_path, src);
        self.load_png(&path)
    }

    pub fn load_png(&self, path: &Path) -> Result<LoadedImage> {
        let image = ImageReader::open(path)?.with_guessed_format()?.decode()?;
        let (width, height) = image.dimensions();
        let png_bytes = if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("png")) {
            fs::read(path)?
        } else {
            let mut cursor = Cursor::new(Vec::new());
            image.write_to(&mut cursor, ImageFormat::Png)?;
            cursor.into_inner()
        };

        Ok(LoadedImage { path: path.to_path_buf(), width, height, png_bytes })
    }

    pub fn dimensions_from_png_bytes(&self, bytes: &[u8]) -> Result<(u32, u32)> {
        let image = image::load_from_memory(bytes)?;
        Ok(image.dimensions())
    }
}

#[must_use]
pub fn resolve_image_path(document_path: &Path, src: &str) -> PathBuf {
    let source_path = Path::new(src);
    if source_path.is_absolute() {
        return source_path.to_path_buf();
    }

    document_path
        .parent()
        .map_or_else(|| source_path.to_path_buf(), |parent| parent.join(source_path))
}
