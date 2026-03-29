use mdv::io::self_update::{main_channel_asset_url, release_asset_name};

#[test]
fn maps_linux_and_macos_targets_to_release_assets() {
    assert_eq!(
        release_asset_name("linux", "x86_64"),
        Some("mdv-x86_64-unknown-linux-gnu.tar.gz".to_string())
    );
    assert_eq!(
        release_asset_name("linux", "aarch64"),
        Some("mdv-aarch64-unknown-linux-gnu.tar.gz".to_string())
    );
    assert_eq!(
        release_asset_name("macos", "x86_64"),
        Some("mdv-x86_64-apple-darwin.tar.gz".to_string())
    );
    assert_eq!(
        release_asset_name("macos", "aarch64"),
        Some("mdv-aarch64-apple-darwin.tar.gz".to_string())
    );
}

#[test]
fn rejects_unsupported_release_targets() {
    assert_eq!(release_asset_name("windows", "x86_64"), None);
    assert_eq!(release_asset_name("linux", "powerpc"), None);
}

#[test]
fn builds_main_channel_asset_urls() {
    assert_eq!(
        main_channel_asset_url("linux", "x86_64"),
        Some(
            "https://github.com/posaune0423/mdv/releases/download/main/mdv-x86_64-unknown-linux-gnu.tar.gz"
                .to_string()
        )
    );
    assert_eq!(
        main_channel_asset_url("macos", "aarch64"),
        Some(
            "https://github.com/posaune0423/mdv/releases/download/main/mdv-aarch64-apple-darwin.tar.gz"
                .to_string()
        )
    );
}
