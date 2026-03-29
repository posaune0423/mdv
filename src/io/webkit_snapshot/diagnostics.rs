use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct SnapshotResult {
    pub png_bytes: Vec<u8>,
    pub diagnostics: SnapshotDiagnostics,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDiagnostics {
    #[serde(default)]
    pub fonts_ready: bool,
    #[serde(default)]
    pub prose_font_ready: bool,
    #[serde(default)]
    pub images_ready: bool,
    #[serde(default)]
    pub mermaids_ready: bool,
    #[serde(default)]
    pub heading_font_weight: String,
    #[serde(default)]
    pub strong_font_weight: String,
    #[serde(default)]
    pub typography: Vec<SnapshotTypographyDiagnostics>,
    #[serde(default)]
    pub images: Vec<SnapshotAssetDiagnostics>,
    #[serde(default)]
    pub mermaids: Vec<SnapshotAssetDiagnostics>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotAssetDiagnostics {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub current_src: String,
    #[serde(default)]
    pub complete: bool,
    #[serde(default, rename = "naturalWidth")]
    pub natural_width_px: f64,
    #[serde(default, rename = "naturalHeight")]
    pub natural_height_px: f64,
    #[serde(default, rename = "renderedWidth")]
    pub rendered_width_px: f64,
    #[serde(default, rename = "renderedHeight")]
    pub rendered_height_px: f64,
    #[serde(default)]
    pub view_box: String,
    #[serde(default)]
    pub content_length: usize,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotTypographyDiagnostics {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub present: bool,
    #[serde(default)]
    pub font_family: String,
    #[serde(default)]
    pub font_weight: String,
    #[serde(default)]
    pub font_style: String,
    #[serde(default, rename = "fontSize")]
    pub font_size_px: f64,
    #[serde(default, rename = "lineHeight")]
    pub line_height_px: f64,
}
