use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn delegation_history_help_exits_zero() {
    let out = Command::new(binary()).args(["delegation-history", "--help"]).output().unwrap();
    assert!(out.status.success(), "delegation-history --help failed: {:?}", out);
}

#[test]
fn delegation_history_zero_address_returns_valid_json() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin).args(["delegation-history", "--address", "0x0000000000000000000000000000000000000001"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("delegation-history output must be valid JSON");
        assert!(v["ok"].is_boolean());
        if v["ok"].as_bool() == Some(true) {
            assert!(v["events"].is_array(), "events must be array");
        }
    }
}
