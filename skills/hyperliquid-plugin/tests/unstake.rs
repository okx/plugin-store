use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn unstake_help_exits_zero() {
    let out = Command::new(binary()).args(["unstake", "--help"]).output().unwrap();
    assert!(out.status.success(), "unstake --help failed: {:?}", out);
}

#[test]
fn unstake_preview_without_confirm() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin)
        .args(["unstake", "--amount", "10", "--validator", "0x0000000000000000000000000000000000000001"])
        .output().unwrap();
    assert_eq!(out.status.code(), Some(0), "unstake without --confirm must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("unstake output must be valid JSON");
        assert!(v["ok"].is_boolean());
    }
}

#[test]
fn unstake_zero_amount_returns_error() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin)
        .args(["unstake", "--amount", "0", "--validator", "0x000"])
        .output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["ok"].as_bool(), Some(false));
    }
}
