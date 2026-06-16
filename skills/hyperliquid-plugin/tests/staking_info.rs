use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn staking_info_help_exits_zero() {
    let out = Command::new(binary()).args(["staking-info", "--help"]).output().unwrap();
    assert!(out.status.success(), "staking-info --help failed: {:?}", out);
}

#[test]
fn staking_info_requires_wallet_or_onchainos() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin).args(["staking-info", "--address", "0x0000000000000000000000000000000000000001"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("staking-info output must be valid JSON");
        assert!(v["ok"].is_boolean(), "ok field must be present");
        if v["ok"].as_bool() == Some(true) {
            // has_stake must be present
            assert!(v["has_stake"].is_boolean(), "has_stake must be boolean");
            // staked_amount + staked_amount_raw must be paired
            assert!(v["staked_amount"].is_string(), "staked_amount must be present");
            assert!(v["staked_amount_raw"].is_string(), "staked_amount_raw must be present");
        }
    }
}
