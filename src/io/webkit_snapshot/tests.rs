use std::path::Path;

use super::{
    SnapshotDiagnostics,
    paths::{create_workspace, snapshot_asset_root},
};

#[test]
fn diagnostics_default_to_empty_assets() {
    let diagnostics = SnapshotDiagnostics::default();

    assert!(diagnostics.images.is_empty());
    assert!(diagnostics.mermaids.is_empty());
}

#[test]
fn snapshot_asset_root_expands_to_cover_relative_parent_assets() {
    let html = r#"<html><body><img src="../../../../examples/pixel.png" /></body></html>"#;
    let base_dir = Path::new("/Users/example/project/tests/fixtures/gfm/image");

    let root = snapshot_asset_root(html, base_dir);

    assert_eq!(root, Path::new("/Users/example/project"));
}

#[test]
fn snapshot_workspace_uses_temp_dir() {
    let workspace = create_workspace(Path::new("/tmp"), Path::new("/"))
        .unwrap_or_else(|error| panic!("workspace should be created: {error}"));

    assert!(workspace.starts_with(Path::new("/tmp")));
    let _ = std::fs::remove_dir_all(workspace);
}
