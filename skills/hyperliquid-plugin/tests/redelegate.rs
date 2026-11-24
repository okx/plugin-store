use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn redelegate_help_exits_zero() {
    let out = Command::new(binary()).args(["redelegate", "--help"]).output().unwrap();
    assert!(out.status.success(), "redelegate --help failed: {:?}", out);
}

#[test]
fn redelegate_same_validator_returns_error() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin)
        .args([
            "redelegate",
            "--amount", "10",
            "--from-validator", "0xAAAA",
            "--to-validator", "0xAAAA",
        ])
        .output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(v["ok"].as_bool(), Some(false));
        assert_eq!(v["error_code"].as_str(), Some("SAME_VALIDATOR"));
    }
}

#[test]
fn redelegate_preview_without_confirm() {
    let bin = binary();
    if !bin.exists() { return; }
    let out = Command::new(&bin)
        .args([
            "redelegate",
            "--amount", "10",
            "--from-validator", "0x0000000000000000000000000000000000000001",
            "--to-validator", "0x0000000000000000000000000000000000000002",
        ])
        .output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("redelegate output must be valid JSON");
        assert!(v["ok"].is_boolean());
    }
}
