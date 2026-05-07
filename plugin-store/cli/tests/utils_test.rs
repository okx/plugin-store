use plugin_store::utils::platform::current_target;

#[test]
fn test_current_target_returns_valid_triple() {
    let target = current_target();
    let valid = [
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "armv7-unknown-linux-gnueabihf",
    ];
    assert!(valid.contains(&target.as_str()), "Unexpected target: {}", target);
}
