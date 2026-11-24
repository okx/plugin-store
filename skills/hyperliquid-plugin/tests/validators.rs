use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    if p.ends_with("deps") { p.pop(); }
    p.join("hyperliquid-plugin")
}

#[test]
fn validators_help_exits_zero() {
    let out = Command::new(binary()).args(["validators", "--help"]).output().unwrap();
    assert!(out.status.success(), "validators --help failed: {:?}", out);
}

#[test]
fn validators_output_is_valid_json_or_error() {
    let bin = binary();
    if !bin.exists() {
        return; // skip if not built yet
    }
    let out = Command::new(&bin).arg("validators").output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout.trim().is_empty() {
        let v: serde_json::Value = serde_json::from_str(stdout.trim())
            .expect("validators output must be valid JSON");
        // Either ok=true with validators array, or ok=false with error_code
        assert!(v["ok"].is_boolean(), "ok field must be present");
        if v["ok"].as_bool() == Some(true) {
            assert!(v["validators"].is_array(), "validators field must be array");
        } else {
            assert!(v["error_code"].is_string(), "error_code must be present on failure");
        }
    }
}
