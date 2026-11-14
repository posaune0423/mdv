//! Best-effort, versioned storage of rendered RGBA pages.
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{core::config::AppConfig, ui::page_graphics::GraphicPage};

const MAGIC: &[u8; 8] = b"MDVRGBA1";
const ENTRY_LIMIT: u64 = 128 * 1024 * 1024;
const TOTAL_LIMIT: u64 = 512 * 1024 * 1024;
const REMOTE_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Request {
    source: String,
    document: PathBuf,
    base: PathBuf,
    theme: String,
    mermaid: String,
    mermaid_command: Option<String>,
    executable: PathBuf,
    executable_len: u64,
    executable_modified: SystemTime,
    width_cells: u16,
    display_width_px: u32,
    snapshot_width_px: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Dependency {
    path: PathBuf,
    fingerprint: u64,
}

#[derive(Serialize, Deserialize)]
struct Header {
    request: Request,
    dependencies: Vec<Dependency>,
    remote: bool,
    created: SystemTime,
    display_height_px: u32,
    image_width_px: u32,
    image_height_px: u32,
    pixels_hash: u64,
}

pub(crate) struct PageCache {
    root: PathBuf,
    request: Request,
    key: u64,
    prepared: Option<(u64, Vec<Dependency>, bool)>,
}

impl PageCache {
    pub(crate) fn for_document(
        config: &AppConfig,
        source_text: &str,
        width_cells: u16,
        display_width_px: u32,
        snapshot_width_px: u32,
    ) -> Option<Self> {
        let executable = fs::canonicalize(std::env::current_exe().ok()?).ok()?;
        let metadata = fs::metadata(&executable).ok()?;
        let absolute = std::env::current_dir().ok()?.join(&config.path);
        let request = Request {
            source: source_text.to_owned(),
            document: fs::canonicalize(&config.path).ok()?,
            base: fs::canonicalize(absolute.parent()?).ok()?,
            theme: config.theme.resolve().as_str().to_owned(),
            mermaid: format!("{:?}", config.mermaid_mode),
            mermaid_command: std::env::var_os("MDV_MERMAID_CMD")
                .map(|value| value.into_string())
                .transpose()
                .ok()?,
            executable,
            executable_len: metadata.len(),
            executable_modified: metadata.modified().ok()?,
            width_cells,
            display_width_px,
            snapshot_width_px,
        };
        let key = hash(&serde_json::to_vec(&request).ok()?);
        Some(Self { root: cache_root()?, request, key, prepared: None })
    }

    fn path(&self) -> PathBuf {
        self.root.join(format!("{:016x}.rgba", self.key))
    }

    pub(crate) fn load(&self) -> Option<GraphicPage> {
        if !private_metadata(&self.root)?.is_dir() || !private_metadata(&self.path())?.is_file() {
            return None;
        }
        let mut file = File::open(self.path()).ok()?;
        let size = file.metadata().ok()?.len();
        if !(16..=ENTRY_LIMIT).contains(&size) {
            return None;
        }
        let mut magic = [0; 8];
        file.read_exact(&mut magic).ok()?;
        if &magic != MAGIC {
            return None;
        }
        let mut length = [0; 8];
        file.read_exact(&mut length).ok()?;
        let header_length = u64::from_le_bytes(length);
        if header_length > size - 16 {
            return None;
        }
        let mut encoded = vec![0; usize::try_from(header_length).ok()?];
        file.read_exact(&mut encoded).ok()?;
        let header: Header = serde_json::from_slice(&encoded).ok()?;
        let raster_scale = header.image_width_px as f32 / self.request.display_width_px as f32;
        let expected_height =
            (header.image_height_px as f32 / raster_scale).round().max(1.0) as u32;
        if header.request != self.request || header.display_height_px != expected_height {
            return None;
        }
        if header.remote && header.created.elapsed().ok()? > REMOTE_TTL {
            return None;
        }
        let pixels_length = pixel_length(header.image_width_px, header.image_height_px)?;
        if pixels_length != size - 16 - header_length {
            return None;
        }
        if header.request.width_cells == 0 || header.request.display_width_px == 0 {
            return None;
        }
        for dependency in &header.dependencies {
            if fingerprint(&dependency.path)? != dependency.fingerprint {
                return None;
            }
        }
        let mut pixels = vec![0; usize::try_from(pixels_length).ok()?];
        file.read_exact(&mut pixels).ok()?;
        if pixels_hash(header.image_width_px, header.image_height_px, &pixels) != header.pixels_hash
        {
            return None;
        }
        Some(GraphicPage {
            width_cells: self.request.width_cells,
            display_width_px: self.request.display_width_px,
            display_height_px: header.display_height_px,
            image_width_px: header.image_width_px,
            image_height_px: header.image_height_px,
            image: Arc::new(RgbaImage::from_raw(
                header.image_width_px,
                header.image_height_px,
                pixels,
            )?),
        })
    }

    pub(crate) fn prepare(&mut self, html: &str) {
        self.prepared = dependencies(html, &self.request.base)
            .map(|(dependencies, remote)| (hash(html.as_bytes()), dependencies, remote));
    }

    pub(crate) fn store(&self, html: &str, page: &GraphicPage) {
        let _ = self.try_store(html, page);
    }

    fn try_store(&self, html: &str, page: &GraphicPage) -> Option<()> {
        if html.contains("mdv-mermaid-fallback\">Mermaid unavailable</div>") {
            return None;
        }
        if page.width_cells != self.request.width_cells
            || page.display_width_px != self.request.display_width_px
            || page.width_cells == 0
            || page.display_width_px == 0
            || page.display_height_px == 0
            || page.image.dimensions() != (page.image_width_px, page.image_height_px)
        {
            return None;
        }
        let pixels_length = pixel_length(page.image_width_px, page.image_height_px)?;
        let (html_hash, dependencies_before, remote_before) = self.prepared.as_ref()?;
        let (dependencies, remote) = dependencies(html, &self.request.base)?;
        if *html_hash != hash(html.as_bytes())
            || *dependencies_before != dependencies
            || *remote_before != remote
        {
            return None;
        }
        let header = Header {
            request: self.request.clone(),
            dependencies,
            remote,
            created: SystemTime::now(),
            display_height_px: page.display_height_px,
            image_width_px: page.image_width_px,
            image_height_px: page.image_height_px,
            pixels_hash: pixels_hash(
                page.image_width_px,
                page.image_height_px,
                page.image.as_raw(),
            ),
        };
        let encoded = serde_json::to_vec(&header).ok()?;
        if 16 + encoded.len() as u64 + pixels_length > ENTRY_LIMIT {
            return None;
        }
        private_directory(&self.root)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&self.root).ok()?;
        temporary.write_all(MAGIC).ok()?;
        temporary.write_all(&(encoded.len() as u64).to_le_bytes()).ok()?;
        temporary.write_all(&encoded).ok()?;
        temporary.write_all(page.image.as_raw()).ok()?;
        temporary.flush().ok()?;
        temporary.persist(self.path()).ok()?;
        cleanup(&self.root);
        Some(())
    }
}

fn cache_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("MDV_CACHE_DIR") {
        return Some(root.into());
    }
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("mdv/pages"));
    }
    if let Some(root) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(root).join("mdv/pages"));
    }
    let home = PathBuf::from(std::env::var_os("HOME")?);
    Some(home.join(if cfg!(target_os = "macos") {
        "Library/Caches/mdv/pages"
    } else {
        ".cache/mdv/pages"
    }))
}

fn private_directory(root: &Path) -> Option<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(root).ok()?;
    private_metadata(root)?.is_dir().then_some(())
}

fn private_metadata(path: &Path) -> Option<fs::Metadata> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return None;
        }
    }
    Some(metadata)
}

fn pixels_hash(width: u32, height: u32, pixels: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    (width, height, pixels).hash(&mut hasher);
    hasher.finish()
}

fn pixel_length(width: u32, height: u32) -> Option<u64> {
    let length = u64::from(width).checked_mul(u64::from(height))?.checked_mul(4)?;
    (length > 0 && length <= ENTRY_LIMIT).then_some(length)
}

fn hash(value: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn fingerprint(path: &Path) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut hasher = DefaultHasher::new();
    let mut buffer = [0; 16384];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.write(&buffer[..read]);
    }
    Some(hasher.finish())
}

fn cleanup(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()?.to_str()? != "rgba" {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            Some((metadata.modified().unwrap_or(UNIX_EPOCH), metadata.len(), path))
        })
        .collect();
    files.sort_by_key(|(modified, _, _)| *modified);
    let mut total: u64 = files.iter().map(|(_, size, _)| size).sum();
    for (_, size, path) in files {
        if total <= TOTAL_LIMIT {
            break;
        }
        if fs::remove_file(path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
}

fn dependencies(html: &str, directory: &Path) -> Option<(Vec<Dependency>, bool)> {
    let base = Url::from_directory_path(directory).ok()?;
    let mut dependencies = Vec::new();
    let mut remote = false;
    // The renderer normalizes image attributes to double-quoted HTML. Unknown
    // quoting or source-set syntax is deliberately a cache miss.
    for attribute in ["src", "srcset"] {
        let needle = format!("{attribute}=");
        for (index, tail) in html.match_indices(&needle) {
            if index > 0 && !html.as_bytes()[index - 1].is_ascii_whitespace() {
                continue;
            }
            let value = html.get(index + tail.len()..)?.strip_prefix('"')?.split_once('"')?.0;
            let value = decode_entities(value)?;
            let sources = if attribute == "srcset" {
                if value.contains("data:") {
                    return None;
                }
                let mut sources = Vec::new();
                for candidate in value.split(',') {
                    let parts: Vec<_> = candidate.split_whitespace().collect();
                    if parts.is_empty() || parts.len() > 2 {
                        return None;
                    }
                    if parts.len() == 2 {
                        let descriptor = parts[1];
                        let number = descriptor
                            .strip_suffix('w')
                            .or_else(|| descriptor.strip_suffix('x'))?;
                        if number.parse::<f64>().ok()? <= 0.0 {
                            return None;
                        }
                    }
                    sources.push(parts[0]);
                }
                sources
            } else {
                vec![value.as_str()]
            };
            for source in sources {
                let url = base.join(source).ok()?;
                match url.scheme() {
                    "data" => {}
                    "http" | "https" => remote = true,
                    "file" => {
                        let path = url.to_file_path().ok()?;
                        if dependencies
                            .iter()
                            .any(|dependency: &Dependency| dependency.path == path)
                        {
                            continue;
                        }
                        dependencies.push(Dependency { fingerprint: fingerprint(&path)?, path });
                    }
                    _ => return None,
                }
            }
        }
    }
    Some((dependencies, remote))
}

fn decode_entities(value: &str) -> Option<String> {
    let mut result = String::new();
    let mut rest = value;
    while let Some((prefix, entity)) = rest.split_once('&') {
        result.push_str(prefix);
        let Some((name, suffix)) = entity.split_once(';') else {
            result.push('&');
            result.push_str(entity);
            return Some(result);
        };
        let character = match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            name if name.starts_with("#x") || name.starts_with("#X") => {
                char::from_u32(u32::from_str_radix(&name[2..], 16).ok()?)?
            }
            name if name.starts_with('#') => char::from_u32(name[1..].parse().ok()?)?,
            _ => return None,
        };
        result.push(character);
        rest = suffix;
    }
    result.push_str(rest);
    Some(result)
}

#[cfg(test)]
#[path = "page_cache_tests.rs"]
mod tests;
