use super::*;
use crate::{cli::Theme, core::config::MermaidMode};
use image::{Rgba, RgbaImage};
use std::{fs, sync::Arc};

fn fixture() -> anyhow::Result<(tempfile::TempDir, AppConfig, GraphicPage)> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("doc.md");
    fs::write(&path, "# Test")?;
    let config =
        AppConfig { path, watch: false, theme: Theme::Light, mermaid_mode: MermaidMode::Enabled };
    let page = GraphicPage {
        width_cells: 2,
        display_width_px: 2,
        display_height_px: 3,
        image_width_px: 2,
        image_height_px: 3,
        image: Arc::new(RgbaImage::from_pixel(2, 3, Rgba([1, 2, 3, 255]))),
    };
    Ok((dir, config, page))
}
fn cache(dir: &tempfile::TempDir, config: &AppConfig, source: &str) -> anyhow::Result<PageCache> {
    let mut cache = PageCache::for_document(config, source, 2, 2, 2)
        .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
    cache.root = dir.path().join("cache");
    Ok(cache)
}
#[test]
fn reloads_pixels_from_another_instance() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    save(&mut cache(&dir, &config, "# Test")?, "<p>Test</p>", &page);
    assert_eq!(cache(&dir, &config, "# Test")?.load(), Some(page));
    Ok(())
}
#[test]
fn invalidates_content_even_with_same_file_size_and_mtime() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let path = dir.path().join("a & b.png");
    fs::write(&path, "one")?;
    let modified = fs::metadata(&path)?.modified()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "<img src=\"a%20&amp;%20b.png\">", &page);
    assert_eq!(cache.load(), Some(page));
    fs::write(&path, "two")?;
    File::options().write(true).open(&path)?.set_modified(modified)?;
    assert!(cache.load().is_none());
    Ok(())
}
#[test]
fn request_changes_miss() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    save(&mut cache(&dir, &config, "# Test")?, "", &page);
    assert!(cache(&dir, &config, "changed")?.load().is_none());
    let mut changed = config.clone();
    changed.theme = Theme::Dark;
    assert!(cache(&dir, &changed, "# Test")?.load().is_none());
    changed = config.clone();
    changed.path = dir.path().join("other.md");
    fs::write(&changed.path, "# Test")?;
    assert!(cache(&dir, &changed, "# Test")?.load().is_none());
    let mut width = PageCache::for_document(&config, "# Test", 3, 3, 3)
        .ok_or_else(|| anyhow::anyhow!("cache unavailable"))?;
    width.root = dir.path().join("cache");
    assert!(width.load().is_none());
    Ok(())
}
#[test]
fn malformed_entries_and_unwritable_roots_are_misses() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    for entry in fs::read_dir(&cache.root)? {
        fs::write(entry?.path(), "corrupt")?;
    }
    assert!(cache.load().is_none());
    fs::remove_dir_all(&cache.root)?;
    fs::write(&cache.root, "not a directory")?;
    save(&mut cache, "", &page);
    assert!(cache.load().is_none());
    Ok(())
}
#[test]
fn missing_assets_and_mermaid_failures_are_not_cached() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "<img src=\"missing.png\">", &page);
    assert!(cache.load().is_none());
    save(&mut cache, "<div class=\"mdv-mermaid-fallback\">Mermaid unavailable</div>", &page);
    assert!(cache.load().is_none());
    Ok(())
}

#[test]
fn does_not_cache_images_changed_during_rendering() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    fs::write(dir.path().join("asset.png"), "before")?;
    let mut cache = cache(&dir, &config, "# Test")?;
    cache.prepare("<img src=\"asset.png\">");
    fs::write(dir.path().join("asset.png"), "after")?;
    cache.store("<img src=\"asset.png\">", &page);
    assert!(cache.load().is_none());
    Ok(())
}

fn save(cache: &mut PageCache, html: &str, page: &GraphicPage) {
    cache.prepare(html);
    cache.store(html, page);
}

#[test]
fn tracks_picture_sources_and_deleted_assets() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    fs::write(dir.path().join("small.png"), "small")?;
    fs::write(dir.path().join("large.png"), "large")?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, r#"<picture><source srcset="small.png 1x, large.png 2x"></picture>"#, &page);
    assert_eq!(cache.load(), Some(page));
    fs::remove_file(dir.path().join("large.png"))?;
    assert!(cache.load().is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_document_keeps_its_rendering_base() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let other = dir.path().join("linked");
    fs::create_dir(&other)?;
    let mut linked = config.clone();
    linked.path = other.join("doc.md");
    std::os::unix::fs::symlink(&config.path, &linked.path)?;
    fs::write(other.join("image.png"), "image")?;
    let mut cache = cache(&dir, &linked, "# Test")?;
    save(&mut cache, r#"<img src="image.png">"#, &page);
    assert_eq!(cache.load(), Some(page));
    fs::write(other.join("image.png"), "other")?;
    assert!(cache.load().is_none());
    Ok(())
}

fn rewrite_header(cache: &PageCache, edit: impl FnOnce(&mut Header)) -> anyhow::Result<()> {
    let bytes = fs::read(cache.path())?;
    let length = u64::from_le_bytes(bytes[8..16].try_into()?) as usize;
    let mut header: Header = serde_json::from_slice(&bytes[16..16 + length])?;
    edit(&mut header);
    let encoded = serde_json::to_vec(&header)?;
    let mut file = File::create(cache.path())?;
    file.write_all(MAGIC)?;
    file.write_all(&(encoded.len() as u64).to_le_bytes())?;
    file.write_all(&encoded)?;
    file.write_all(&bytes[16 + length..])?;
    Ok(())
}

#[test]
fn remote_assets_expire_but_embedded_assets_do_not() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, r#"<img src="https://example.com/image.png">"#, &page);
    assert_eq!(cache.load(), Some(page.clone()));
    rewrite_header(&cache, |header| header.created = UNIX_EPOCH)?;
    assert!(cache.load().is_none());
    save(&mut cache, r#"<img src="data:image/png;base64,AAAA">"#, &page);
    rewrite_header(&cache, |header| header.created = UNIX_EPOCH)?;
    assert_eq!(cache.load(), Some(page));
    Ok(())
}

#[test]
fn rejects_corrupt_geometry_and_colliding_request() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    rewrite_header(&cache, |header| header.display_height_px = u32::MAX)?;
    assert!(cache.load().is_none());
    save(&mut cache, "", &page);
    rewrite_header(&cache, |header| header.request.source = "collision".to_owned())?;
    assert!(cache.load().is_none());
    save(&mut cache, "", &page);
    rewrite_header(&cache, |header| header.image_width_px = u32::MAX)?;
    assert!(cache.load().is_none());
    Ok(())
}

#[test]
fn uncertain_dependency_syntax_is_not_cached() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    for html in [
        r#"<source srcset="data:image/png;base64,AAAA 1x, missing.png 2x">"#,
        r#"<img src="image&unknown;.png">"#,
        "<img src='image.png'>",
    ] {
        save(&mut cache, html, &page);
        assert!(cache.load().is_none(), "{html}");
    }
    Ok(())
}

#[test]
fn cleanup_evicts_oldest_entries_until_under_budget() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    for index in 0..5 {
        let file = File::create(dir.path().join(format!("{index}.rgba")))?;
        file.set_len(ENTRY_LIMIT)?;
        file.set_modified(UNIX_EPOCH + Duration::from_secs(index))?;
    }
    cleanup(dir.path());
    assert!(!dir.path().join("0.rgba").exists());
    assert!(dir.path().join("1.rgba").exists());
    assert_eq!(fs::read_dir(dir.path())?.count(), 4);
    Ok(())
}

#[cfg(unix)]
#[test]
fn cache_files_are_private() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    assert_eq!(fs::metadata(&cache.root)?.permissions().mode() & 0o777, 0o700);
    assert_eq!(fs::metadata(cache.path())?.permissions().mode() & 0o777, 0o600);
    Ok(())
}

#[cfg(unix)]
#[test]
fn does_not_change_existing_directory_permissions() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    fs::create_dir(&cache.root)?;
    fs::set_permissions(&cache.root, fs::Permissions::from_mode(0o755))?;
    save(&mut cache, "", &page);
    assert_eq!(fs::metadata(&cache.root)?.permissions().mode() & 0o777, 0o755);
    assert!(cache.load().is_none());
    Ok(())
}

#[test]
fn deliberately_disabled_mermaid_is_cacheable() -> anyhow::Result<()> {
    let (dir, mut config, page) = fixture()?;
    config.mermaid_mode = MermaidMode::Disabled;
    let mut cache = cache(&dir, &config, "```mermaid\ngraph TD; A-->B\n```")?;
    save(
        &mut cache,
        r#"<div class="mdv-mermaid mdv-mermaid-fallback">Mermaid disabled</div>"#,
        &page,
    );
    assert_eq!(cache.load(), Some(page));
    Ok(())
}

#[cfg(unix)]
#[test]
fn load_rejects_a_directory_that_is_no_longer_private() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    fs::set_permissions(&cache.root, fs::Permissions::from_mode(0o755))?;
    assert!(cache.load().is_none());
    assert_eq!(fs::metadata(&cache.root)?.permissions().mode() & 0o777, 0o755);
    Ok(())
}

#[cfg(unix)]
#[test]
fn load_rejects_a_symlink_directory() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    let moved = dir.path().join("moved-cache");
    fs::rename(&cache.root, &moved)?;
    std::os::unix::fs::symlink(&moved, &cache.root)?;
    assert!(cache.load().is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn load_rejects_a_non_private_entry() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    fs::set_permissions(cache.path(), fs::Permissions::from_mode(0o644))?;
    assert!(cache.load().is_none());
    Ok(())
}

#[cfg(unix)]
#[test]
fn load_rejects_a_symlink_entry() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    let moved = dir.path().join("moved-entry");
    fs::rename(cache.path(), &moved)?;
    std::os::unix::fs::symlink(&moved, cache.path())?;
    assert!(cache.load().is_none());
    Ok(())
}

#[test]
fn load_does_not_create_missing_cache_directory() -> anyhow::Result<()> {
    let (dir, config, _) = fixture()?;
    let cache = cache(&dir, &config, "# Test")?;
    assert!(cache.load().is_none());
    assert!(!cache.root.exists());
    Ok(())
}

#[test]
fn rejects_reshaped_pixels_with_unchanged_byte_count() -> anyhow::Result<()> {
    let (dir, config, page) = fixture()?;
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    rewrite_header(&cache, |header| {
        header.image_width_px = 3;
        header.image_height_px = 2;
        header.display_height_px = 1;
    })?;
    assert!(cache.load().is_none());
    Ok(())
}

#[test]
fn accepts_device_scaled_rasters() -> anyhow::Result<()> {
    let (dir, config, mut page) = fixture()?;
    page.image_width_px = 4;
    page.image_height_px = 6;
    page.image = Arc::new(RgbaImage::from_pixel(4, 6, Rgba([1, 2, 3, 255])));
    let mut cache = cache(&dir, &config, "# Test")?;
    save(&mut cache, "", &page);
    assert_eq!(cache.load(), Some(page));
    Ok(())
}
