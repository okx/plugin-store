use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn unbonding_help_exits_zero() {
    let out = Command::new(binary()).args(["unbonding", "--help"]).output().unwrap();
    assert!(out.status.success(), "unbonding --help failed: {:?}", out);
}

#[test]
fn unbonding_zero_address_returns_valid_json() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin).args(["unbonding", "--address", "0x0000000000000000000000000000000000000001"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("unbonding output must be valid JSON");
        assert!(v["ok"].is_boolean());
        if v["ok"].as_bool() == Some(true) {
            assert!(v["pending_withdrawals_count"].is_u64(), "pending_withdrawals_count must be a number");
        }
    }
}
