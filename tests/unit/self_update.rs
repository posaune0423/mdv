use mdv::io::self_update::{main_binary_url, main_binary_url_for_target};

#[test]
fn points_update_to_the_tracked_main_binary() {
    assert_eq!(main_binary_url(), "https://raw.githubusercontent.com/posaune0423/mdv/main/bin/mdv");
}

#[test]
fn selects_a_windows_executable_for_windows_x86_64() {
    assert_eq!(
        main_binary_url_for_target("windows", "x86_64").unwrap_or_else(|error| panic!(
            "Windows x86_64 should have an update artifact: {error}"
        )),
        "https://raw.githubusercontent.com/posaune0423/mdv/main/bin/mdv.exe"
    );
}

#[test]
fn rejects_update_artifacts_for_unsupported_targets() {
    let error = match main_binary_url_for_target("linux", "x86_64") {
        Ok(url) => panic!("Linux does not have a tracked update artifact, received {url}"),
        Err(error) => error,
    };

    assert!(
        error.to_string().contains("unsupported update target linux-x86_64"),
        "unexpected error: {error}"
    );
}
