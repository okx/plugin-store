use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn stake_help_exits_zero() {
    let out = Command::new(binary()).args(["stake", "--help"]).output().unwrap();
    assert!(out.status.success(), "stake --help failed: {:?}", out);
}

#[test]
fn stake_preview_without_confirm() {
    let bin = binary();
    if !bin.exists() { return; }
    // Without --confirm, should return ok=true preview (or fail with WALLET_NOT_FOUND / BALANCE_QUERY_FAILED in CI)
    let out = Command::new(&bin)
        .args(["stake", "--amount", "100", "--validator", "0x0000000000000000000000000000000000000001"])
        .output().unwrap();
    assert_eq!(out.status.code(), Some(0), "stake without --confirm must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("stake output must be valid JSON");
        assert!(v["ok"].is_boolean());
    }
}

#[test]
fn stake_zero_amount_returns_error() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin)
        .args(["stake", "--amount", "0", "--validator", "0x000"])
        .output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["ok"].as_bool(), Some(false));
        assert!(v["error_code"].is_string());
    }
}
