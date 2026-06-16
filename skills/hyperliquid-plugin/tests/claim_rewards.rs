use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn claim_rewards_help_exits_zero() {
    let out = Command::new(binary()).args(["claim-rewards", "--help"]).output().unwrap();
    assert!(out.status.success(), "claim-rewards --help failed: {:?}", out);
}

#[test]
fn claim_rewards_preview_without_confirm() {
    let bin = binary();
    if !bin.exists() { return; }
    // Without --confirm, should return ok=true preview or an error (no rewards / wallet not found)
    let out = Command::new(&bin).args(["claim-rewards"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0), "claim-rewards without --confirm must exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("claim-rewards output must be valid JSON");
        assert!(v["ok"].is_boolean());
        if v["ok"].as_bool() == Some(false) {
            assert!(v["error_code"].is_string());
        }
    }
}
