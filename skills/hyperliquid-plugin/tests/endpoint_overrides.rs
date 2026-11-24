// The test-injection overrides introduced for the autotrade gate must be invisible in
// production: with no override set, every endpoint has to resolve exactly as it did
// before they existed. These assertions are the guard against a testability seam
// quietly changing where real orders are sent.
//
// Each case runs in its own process because the resolved values are cached per process.

use std::process::Command;

fn binary() -> std::path::PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    if p.ends_with("deps") {
        p.pop();
    }
    p.join("hyperliquid-plugin")
}

/// `prices` prints the endpoint it queried, which is the cheapest observable proof of
/// which base URL the binary resolved.
fn resolved_endpoint(env: &[(&str, &str)]) -> String {
    let mut cmd = Command::new(binary());
    cmd.args(["prices", "--coin", "BTC"]);
    // Inherit nothing that could preselect an endpoint.
    cmd.env_remove("HYPERLIQUID_TEST_MODE");
    cmd.env_remove("HYPERLIQUID_TEST_API_BASE");
    cmd.env_remove("HYPERLIQUID_TESTNET");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("spawn plugin");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn mainnet_endpoint_unchanged_without_override() {
    let out = resolved_endpoint(&[]);
    assert!(
        out.contains("api.hyperliquid.xyz"),
        "mainnet must still resolve to the production host: {}",
        out
    );
    assert!(
        !out.contains("hyperliquid-testnet"),
        "mainnet must not resolve to testnet: {}",
        out
    );
}

#[test]
fn testnet_endpoint_unchanged_without_override() {
    let out = resolved_endpoint(&[("HYPERLIQUID_TESTNET", "1")]);
    assert!(
        out.contains("api.hyperliquid-testnet.xyz"),
        "testnet flag must still resolve to the testnet host: {}",
        out
    );
}

#[test]
fn override_redirects_when_set() {
    let out = resolved_endpoint(&[
        ("HYPERLIQUID_TEST_MODE", "1"),
        ("HYPERLIQUID_TEST_API_BASE", "http://127.0.0.1:1"),
    ]);
    assert!(
        out.contains("127.0.0.1:1"),
        "override must be honoured when set: {}",
        out
    );
    assert!(
        !out.contains("api.hyperliquid.xyz"),
        "override must replace the production host, not supplement it: {}",
        out
    );
}

#[test]
fn empty_override_falls_back_to_production() {
    let out = resolved_endpoint(&[
        ("HYPERLIQUID_TEST_MODE", "1"),
        ("HYPERLIQUID_TEST_API_BASE", ""),
    ]);
    assert!(
        out.contains("api.hyperliquid.xyz"),
        "an empty override must not blank out the endpoint: {}",
        out
    );
}

#[test]
fn override_is_ignored_without_test_mode() {
    let out = resolved_endpoint(&[("HYPERLIQUID_TEST_API_BASE", "http://127.0.0.1:1")]);
    assert!(
        out.contains("api.hyperliquid.xyz"),
        "production mode must ignore test endpoint overrides: {}",
        out
    );
    assert!(
        !out.contains("127.0.0.1:1"),
        "test endpoint must not activate without HYPERLIQUID_TEST_MODE: {}",
        out
    );
}
