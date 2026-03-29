use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;

pub(super) fn create_workspace(base_dir: &Path, read_access_root: &Path) -> Result<PathBuf> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let normalized_root = normalize_path(read_access_root);
    let base_candidate = absolutize_path(base_dir);
    for candidate in [base_candidate, normalize_path(&std::env::temp_dir())] {
        if !candidate.starts_with(&normalized_root) {
            continue;
        }

        let dir = candidate.join(".mdv-webkit").join(format!("{timestamp}"));
        if fs::create_dir_all(&dir).is_ok() {
            return Ok(dir);
        }
    }

    let dir = normalized_root.join(".mdv-webkit").join(format!("{timestamp}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub(super) fn common_read_access_root(html: &str, base_dir: &Path) -> PathBuf {
    snapshot_asset_root(html, base_dir)
}

pub(super) fn snapshot_asset_root(html: &str, base_dir: &Path) -> PathBuf {
    let base_dir = absolutize_path(base_dir);
    let mut root = normalize_path(&base_dir);
    for reference in local_asset_references(html) {
        let resolved = resolve_local_reference(&base_dir, &reference);
        root = common_path_prefix(&root, &resolved);
    }
    root
}

fn local_asset_references(html: &str) -> Vec<String> {
    let mut references = Vec::new();
    let mut rest = html;
    while let Some(index) = rest.find(r#"src=""#) {
        rest = &rest[index + 5..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let candidate = &rest[..end];
        if is_local_asset_reference(candidate) {
            references.push(candidate.to_string());
        }
        rest = &rest[end + 1..];
    }
    references
}

fn is_local_asset_reference(reference: &str) -> bool {
    !reference.is_empty()
        && !reference.starts_with("data:")
        && !reference.starts_with("http://")
        && !reference.starts_with("https://")
        && !reference.starts_with('#')
}

fn resolve_local_reference(base_dir: &Path, reference: &str) -> PathBuf {
    if let Some(path) = reference.strip_prefix("file://") {
        return normalize_path(Path::new(path));
    }

    let path = Path::new(reference);
    if path.is_absolute() { normalize_path(path) } else { normalize_path(&base_dir.join(path)) }
}

fn absolutize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return normalize_path(path);
    }

    std::env::current_dir()
        .map(|current_dir| normalize_path(&current_dir.join(path)))
        .unwrap_or_else(|_| normalize_path(path))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = if path.is_absolute() { PathBuf::from("/") } else { PathBuf::new() };
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let _ = normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized }
}

fn common_path_prefix(left: &Path, right: &Path) -> PathBuf {
    let mut prefix = PathBuf::new();
    for (left_component, right_component) in left.components().zip(right.components()) {
        if left_component != right_component {
            break;
        }
        prefix.push(left_component.as_os_str());
    }

    if prefix.as_os_str().is_empty() {
        if left.is_absolute() && right.is_absolute() {
            PathBuf::from("/")
        } else {
            PathBuf::from(".")
        }
    } else {
        prefix
    }
}
