use mdv::io::self_update::main_binary_url;

#[test]
fn points_update_to_the_tracked_main_binary() {
    assert_eq!(main_binary_url(), "https://raw.githubusercontent.com/posaune0423/mdv/main/bin/mdv");
}
