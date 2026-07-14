// Integration tests: --autotrade-job authorization gate (Dev PRD FR-1, AC-1..7).
//
// These tests spawn the compiled `polymarket-plugin` binary as a subprocess with a
// fully hermetic environment (env_clear + per-test temp HOME + wiremock servers),
// so they run in parallel without the process-global env-var mutex and exercise the
// real CLI surface including clap parsing of `--autotrade-job`.
//
// Coverage:
//   AC-1 — grant ok → grant-check called with the frozen FR-3 args, flow reaches the
//          post_order mock, success output carries `autotradeJob`
//   AC-2 — grant deny → fail-closed BEFORE any credential derivation / signing,
//          reason passed through verbatim
//   AC-3 — missing binary / hang (timeout) / invalid JSON / old CLI → same fail-closed
//          behaviour with a failure-mode-specific reason
//   AC-4 — without --autotrade-job the output is byte-identical to 0.6.2 and the
//          grant check is never invoked
//   AC-5 — invalid jobId (shell metacharacters / overlong) → invalid_input, no
//          subprocess spawned
//   AC-6 — --autotrade-job + --dry-run → grant check skipped, dry-run output notes it
//   AC-7 — sell path → grant-check receives --action sell --amount <shares>

mod common;

use common::{
    clob_market, clob_orderbook, mock_onchainos_path, read_call_log,
    rpc_eth_call_u256, rpc_pol_balance, OnchainosCall, TEST_CONDITION_ID, TEST_TX_HASH,
    TEST_WALLET,
};
use serde_json::{json, Value};
use std::process::Output;
use wiremock::matchers::{body_partial_json, body_string_contains, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const JOB_ID: &str = "0xabc";
// Valid base64url — l2 HMAC signing decodes the secret before signing the /order request.
const TEST_API_SECRET: &str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";

// ── Subprocess harness ────────────────────────────────────────────────────────

/// Mock servers + temp HOME + call log for one plugin subprocess invocation.
struct Harness {
    /// Serves CLOB / Gamma / Data / Web (geoblock) — routed by path.
    api_server: MockServer,
    /// Serves Polygon JSON-RPC — routed by request-body content.
    rpc_server: MockServer,
    home: tempfile::TempDir,
    call_log: tempfile::NamedTempFile,
}

impl Harness {
    async fn new() -> Self {
        // Anchor temp files under CARGO_TARGET_TMPDIR — always writable wherever
        // cargo can build (system temp dirs may be blocked in sandboxed CI).
        let tmp_root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
        std::fs::create_dir_all(tmp_root).expect("create target tmpdir");
        Harness {
            api_server: MockServer::start().await,
            rpc_server: MockServer::start().await,
            home: tempfile::tempdir_in(tmp_root).expect("temp home"),
            call_log: tempfile::NamedTempFile::new_in(tmp_root).expect("temp call log"),
        }
    }

    /// Geoblock probe → accessible (the region pre-flight runs before the gate).
    async fn mock_region_accessible(&self) {
        Mock::given(method("GET"))
            .and(path("/api/geoblock"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "blocked": false })))
            .mount(&self.api_server)
            .await;
    }

    /// Public CLOB market-data phase: /markets/{id}, /book, /version (V1).
    async fn mock_market_data(&self) {
        Mock::given(method("GET"))
            .and(path_regex("^/markets/"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(clob_market(TEST_CONDITION_ID, false)),
            )
            .mount(&self.api_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/book"))
            .respond_with(ResponseTemplate::new(200).set_body_json(clob_orderbook()))
            .mount(&self.api_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "version": 1 })))
            .mount(&self.api_server)
            .await;
    }

    /// On-chain reads for the V1 EOA order path: big token balances, MAX allowance,
    /// isApprovedForAll = true, 1 POL for gas.
    async fn mock_rpc_order_path(&self) {
        // isApprovedForAll (0xe985e9c5) → true — mount before the generic eth_call mock.
        Mock::given(method("POST"))
            .and(body_string_contains("e985e9c5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(rpc_eth_call_u256(1)))
            .mount(&self.rpc_server)
            .await;
        // eth_getBalance (POL gas) → 1 POL
        Mock::given(method("POST"))
            .and(body_partial_json(json!({ "method": "eth_getBalance" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(rpc_pol_balance(1.0)))
            .mount(&self.rpc_server)
            .await;
        // Any other eth_call (balanceOf / allowance) → a comfortably large value
        Mock::given(method("POST"))
            .and(body_partial_json(json!({ "method": "eth_call" })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(rpc_eth_call_u256(1_000_000_000_000)),
            )
            .mount(&self.rpc_server)
            .await;
    }

    /// CLOB order placement → success (the post_order mock AC-1 asserts against).
    /// Field names follow the real CLOB response schema (`orderID`, camelCase).
    async fn mock_post_order(&self) {
        Mock::given(method("POST"))
            .and(path("/order"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "orderID": "test-order-12345",
                "status": "matched",
                "makingAmount": "500000",
                "takingAmount": "750000",
            })))
            .mount(&self.api_server)
            .await;
    }

    /// CLOB CONDITIONAL balance for the sell pre-flight (raw share units).
    async fn mock_ctf_balance(&self, balance_raw: u64) {
        Mock::given(method("GET"))
            .and(path("/balance-allowance"))
            .and(query_param("asset_type", "CONDITIONAL"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "asset_address": null,
                "balance": balance_raw.to_string(),
                "allowance": "0",
            })))
            .mount(&self.api_server)
            .await;
    }

    /// Run the plugin binary with a hermetic environment. `grant_result` drives the
    /// mock grant-check ("ok" / "deny" / "hang" / "badjson"); None simulates an old
    /// onchainos that does not know the subcommand. `env_creds` short-circuits
    /// ensure_credentials via POLYMARKET_API_KEY/SECRET/PASSPHRASE (EOA mode).
    fn run_plugin(
        &self,
        args: &[&str],
        grant_result: Option<&str>,
        env_creds: bool,
        extra_env: &[(&str, &str)],
    ) -> Output {
        let bin = env!("CARGO_BIN_EXE_polymarket-plugin");
        let mut cmd = std::process::Command::new(bin);
        cmd.args(args);
        cmd.env_clear();
        // PATH is required so the mock script's `#!/usr/bin/env bash` resolves.
        if let Ok(path_var) = std::env::var("PATH") {
            cmd.env("PATH", path_var);
        }
        cmd.env("HOME", self.home.path());
        cmd.env("POLYMARKET_TEST_POLYGON_RPC", self.rpc_server.uri());
        cmd.env("POLYMARKET_TEST_CLOB_URL", self.api_server.uri());
        cmd.env("POLYMARKET_TEST_GAMMA_URL", self.api_server.uri());
        cmd.env("POLYMARKET_TEST_DATA_URL", self.api_server.uri());
        cmd.env("POLYMARKET_TEST_WEB_URL", self.api_server.uri());
        cmd.env("POLYMARKET_ONCHAINOS_BIN", mock_onchainos_path());
        cmd.env("MOCK_ONCHAINOS_CALL_LOG", self.call_log.path());
        cmd.env("MOCK_ONCHAINOS_WALLET", TEST_WALLET);
        cmd.env("MOCK_ONCHAINOS_TX_HASH", TEST_TX_HASH);
        if let Some(result) = grant_result {
            cmd.env("MOCK_ONCHAINOS_GRANT_RESULT", result);
        }
        if env_creds {
            cmd.env("POLYMARKET_API_KEY", "test-api-key");
            cmd.env("POLYMARKET_SECRET", TEST_API_SECRET);
            cmd.env("POLYMARKET_PASSPHRASE", "test-passphrase");
        }
        for (k, v) in extra_env {
            cmd.env(k, v);
        }
        cmd.output().expect("failed to run polymarket-plugin binary")
    }

    fn calls(&self) -> Vec<OnchainosCall> {
        read_call_log(self.call_log.path())
    }
}

fn stdout_json(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!(
            "plugin stdout is not valid JSON ({}):\nstdout: {}\nstderr: {}",
            e,
            stdout,
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn is_grant_check(call: &OnchainosCall) -> bool {
    call.args.iter().any(|a| a == "agent")
        && call.args.iter().any(|a| a == "autotrade-grant-check")
}

/// Find the grant-check invocation and assert its frozen FR-3 argument contract.
fn assert_grant_check_args(calls: &[OnchainosCall], action: &str, amount: &str) {
    let grant = calls
        .iter()
        .find(|c| is_grant_check(c))
        .expect("grant-check was not invoked");
    let args = &grant.args;
    let pair = |flag: &str, value: &str| {
        let pos = args
            .iter()
            .position(|a| a == flag)
            .unwrap_or_else(|| panic!("grant-check missing {} (args: {:?})", flag, args));
        assert_eq!(
            args.get(pos + 1).map(String::as_str),
            Some(value),
            "grant-check {} should be {} (args: {:?})",
            flag,
            value,
            args
        );
    };
    pair("--job-id", JOB_ID);
    pair("--venue", "polymarket");
    pair("--action", action);
    pair("--amount", amount);
    pair("--format", "json");
}

fn assert_denied(output: &Output, expected_reason_substr: &str) {
    let v = stdout_json(output);
    assert_eq!(v["ok"].as_bool(), Some(false), "expected ok:false, got: {}", v);
    assert_eq!(
        v["error"]["code"].as_str(),
        Some("autotrade_grant_denied"),
        "expected autotrade_grant_denied, got: {}",
        v
    );
    let reason = v["error"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains(expected_reason_substr),
        "reason '{}' should contain '{}'",
        reason,
        expected_reason_substr
    );
}

/// Like `assert_denied` but requires the reason to match EXACTLY — used for the
/// ok:false deny path where the PRD mandates verbatim passthrough of the CLI's
/// reason (a wrapped/prefixed reason must fail this).
fn assert_denied_reason_exact(output: &Output, expected_reason: &str) {
    let v = stdout_json(output);
    assert_eq!(v["ok"].as_bool(), Some(false), "expected ok:false, got: {}", v);
    assert_eq!(v["error"]["code"].as_str(), Some("autotrade_grant_denied"), "got: {}", v);
    assert_eq!(
        v["error"]["reason"].as_str(),
        Some(expected_reason),
        "deny reason must be passed through verbatim, got: {}",
        v
    );
}

/// No credential derivation, signing, or on-chain call may have happened:
/// the call log must contain nothing but the grant check itself.
fn assert_no_auth_or_signing(calls: &[OnchainosCall]) {
    for call in calls {
        assert!(
            is_grant_check(call),
            "unexpected onchainos call before/after a rejected grant check: {:?}",
            call.args
        );
    }
}

// ── AC-1: grant ok → full order path with autotradeJob in the output ─────────

#[tokio::test]
async fn ac1_buy_grant_ok_reaches_post_order_with_autotrade_job() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;
    h.mock_market_data().await;
    h.mock_rpc_order_path().await;
    h.mock_post_order().await;

    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID,
        ],
        Some("ok"),
        true,
        &[],
    );

    let v = stdout_json(&out);
    assert_eq!(v["ok"].as_bool(), Some(true), "buy should succeed: {}", v);
    // post_order mock was reached — its canned order_id round-trips into the output.
    assert_eq!(v["data"]["order_id"].as_str(), Some("test-order-12345"), "output: {}", v);
    assert_eq!(v["data"]["autotradeJob"].as_str(), Some(JOB_ID), "output: {}", v);

    let calls = h.calls();
    assert_grant_check_args(&calls, "buy", "5");

    // The grant check must run before anything else onchainos-related (fail-closed
    // ordering: rejection happens before credential derivation / signing).
    assert!(is_grant_check(&calls[0]), "first onchainos call must be the grant check: {:?}", calls[0].args);
    let sign_pos = calls.iter().position(|c| c.args.iter().any(|a| a == "sign-message"));
    assert!(sign_pos.is_some(), "order signing should have happened after the grant check");
}

// ── AC-2: grant deny → fail-closed, reason passed through verbatim ───────────

#[tokio::test]
async fn ac2_buy_grant_denied_no_auth_no_signing() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID,
        ],
        Some("deny"),
        false,
        &[],
    );

    assert_denied_reason_exact(&out, "per-trade cap exceeded");
    let calls = h.calls();
    assert_grant_check_args(&calls, "buy", "5");
    assert_no_auth_or_signing(&calls);
}

// ── AC-3: missing binary / timeout / invalid JSON / old CLI → fail-closed ────

#[tokio::test]
async fn ac3_missing_onchainos_binary_denies() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID,
        ],
        Some("ok"),
        false,
        &[("POLYMARKET_ONCHAINOS_BIN", "/nonexistent/onchainos-for-test")],
    );

    assert_denied(&out, "could not launch onchainos");
    assert!(h.calls().is_empty(), "no subprocess should have run");
}

#[tokio::test]
async fn ac3_grant_check_timeout_denies() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID,
        ],
        Some("hang"),
        false,
        &[
            ("POLYMARKET_GRANT_CHECK_TIMEOUT_SECS", "1"),
            ("MOCK_ONCHAINOS_GRANT_HANG_SECS", "5"),
        ],
    );

    assert_denied(&out, "grant check timeout");
    let calls = h.calls();
    assert_grant_check_args(&calls, "buy", "5");
    assert_no_auth_or_signing(&calls);
}

#[tokio::test]
async fn ac3_grant_check_invalid_json_denies() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID,
        ],
        Some("badjson"),
        false,
        &[],
    );

    assert_denied(&out, "invalid JSON");
    assert_no_auth_or_signing(&h.calls());
}

/// NFR-3: when onchainos crashes with stderr that references the grant file it was
/// reading, the plugin's denial reason must NOT leak that path (or any stderr).
#[tokio::test]
async fn ac3_crash_stderr_never_leaks_into_denial_reason() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID,
        ],
        Some("crash"),
        false,
        &[],
    );

    assert_denied(&out, "grant check failed");
    let v = stdout_json(&out);
    let reason = v["error"]["reason"].as_str().unwrap_or_default();
    assert!(
        !reason.contains("grants") && !reason.contains(".json") && !reason.contains("panicked"),
        "denial reason must not embed subprocess stderr (grant paths): '{}'",
        reason
    );
    assert_no_auth_or_signing(&h.calls());
}

#[tokio::test]
async fn ac3_old_onchainos_without_subcommand_denies_with_upgrade_hint() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    // grant_result: None → the mock behaves like an old CLI that does not know
    // `agent autotrade-grant-check` (unrecognised-command error on stderr, exit 1).
    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID,
        ],
        None,
        false,
        &[],
    );

    assert_denied(&out, "does not support autotrade-grant-check");
    assert_no_auth_or_signing(&h.calls());
}

// ── AC-4: without --autotrade-job → 0.6.2 behaviour, grant check never runs ──

#[tokio::test]
async fn ac4_no_flag_output_identical_and_grant_check_never_called() {
    let h = Harness::new().await;
    h.mock_market_data().await;

    let args_base = [
        "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
        "--amount", "5", "--dry-run",
    ];
    let out_plain = h.run_plugin(&args_base, Some("ok"), false, &[]);
    let v_plain = stdout_json(&out_plain);

    assert_eq!(v_plain["ok"].as_bool(), Some(true), "dry-run should succeed: {}", v_plain);
    assert!(
        v_plain["data"].get("autotradeGrantCheck").is_none(),
        "no autotrade field may appear without the flag: {}",
        v_plain
    );
    assert!(v_plain["data"].get("autotradeJob").is_none());
    assert!(
        !h.calls().iter().any(is_grant_check),
        "grant check must never be invoked without --autotrade-job"
    );

    // Same command WITH the flag: the output must be the 0.6.2 output plus exactly
    // the one skip-note field (guards accidental drift on the shared path).
    let mut args_flag = args_base.to_vec();
    args_flag.extend_from_slice(&["--autotrade-job", JOB_ID]);
    let out_flag = h.run_plugin(&args_flag, Some("ok"), false, &[]);
    let mut v_flag = stdout_json(&out_flag);
    assert_eq!(
        v_flag["data"]["autotradeGrantCheck"].as_str(),
        Some("skipped (dry-run)")
    );
    v_flag["data"].as_object_mut().unwrap().remove("autotradeGrantCheck");
    assert_eq!(v_flag, v_plain, "flag must add only the autotradeGrantCheck field on dry-run");
}

/// AC-4 on the LIVE order path: without --autotrade-job, a real (non-dry-run) buy
/// runs the full 0.6.2 flow — the grant check is never invoked (even though the
/// mock WOULD approve it), and the success output carries no autotrade fields.
/// This is the mutation guard for "缺省路径零行为变化" that the dry-run variant
/// above cannot provide (dry-run skips the gate even WITH the flag).
#[tokio::test]
async fn ac4_live_no_flag_full_order_path_never_touches_grant_check() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;
    h.mock_market_data().await;
    h.mock_rpc_order_path().await;
    h.mock_post_order().await;

    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5",
        ],
        Some("ok"), // a wrongly-inserted gate would succeed AND appear in the call log
        true,
        &[],
    );

    let v = stdout_json(&out);
    assert_eq!(v["ok"].as_bool(), Some(true), "live no-flag buy should succeed: {}", v);
    assert_eq!(v["data"]["order_id"].as_str(), Some("test-order-12345"), "output: {}", v);
    assert!(
        v["data"].get("autotradeJob").is_none() && v["data"].get("autotradeGrantCheck").is_none(),
        "no autotrade field may appear on the live no-flag path: {}",
        v
    );
    assert!(
        !h.calls().iter().any(is_grant_check),
        "grant check must never be invoked on the live path without --autotrade-job"
    );
}

// ── AC-5: invalid jobId → invalid_input, no subprocess ────────────────────────

#[tokio::test]
async fn ac5_invalid_job_id_rejected_without_subprocess() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    for bad_job in ["abc; rm -rf /", "job id with spaces", "job$(whoami)", &"x".repeat(129), ""] {
        let out = h.run_plugin(
            &[
                "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
                "--amount", "5", "--autotrade-job", bad_job,
            ],
            Some("ok"),
            false,
            &[],
        );
        let v = stdout_json(&out);
        assert_eq!(v["ok"].as_bool(), Some(false), "jobId {:?} must be rejected: {}", bad_job, v);
        assert_eq!(v["error"]["code"].as_str(), Some("invalid_input"), "jobId {:?}: {}", bad_job, v);
        assert_eq!(v["error"]["field"].as_str(), Some("autotrade-job"));
    }
    assert!(
        h.calls().is_empty(),
        "no onchainos subprocess may be spawned for an invalid jobId"
    );
}

#[test]
fn job_id_validation_edges() {
    use polymarket_plugin::onchainos::is_valid_autotrade_job_id;
    assert!(is_valid_autotrade_job_id("0xabc"));
    assert!(is_valid_autotrade_job_id("A-Za_z0-9"));
    assert!(is_valid_autotrade_job_id(&"a".repeat(128)));
    assert!(!is_valid_autotrade_job_id(&"a".repeat(129)));
    assert!(!is_valid_autotrade_job_id(""));
    assert!(!is_valid_autotrade_job_id("abc; rm -rf /"));
    assert!(!is_valid_autotrade_job_id("with space"));
    assert!(!is_valid_autotrade_job_id("newline\n"));
    assert!(!is_valid_autotrade_job_id("unicode—dash"));
}

// ── AC-6: --autotrade-job + --dry-run → grant check skipped ──────────────────

#[tokio::test]
async fn ac6_dry_run_skips_grant_check_and_notes_it() {
    let h = Harness::new().await;
    h.mock_market_data().await;

    // Grant result "deny" proves the check is not even consulted on dry-run.
    let out = h.run_plugin(
        &[
            "buy", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--amount", "5", "--autotrade-job", JOB_ID, "--dry-run",
        ],
        Some("deny"),
        false,
        &[],
    );

    let v = stdout_json(&out);
    assert_eq!(v["ok"].as_bool(), Some(true), "dry-run should succeed: {}", v);
    assert_eq!(v["dry_run"].as_bool(), Some(true));
    assert_eq!(
        v["data"]["autotradeGrantCheck"].as_str(),
        Some("skipped (dry-run)"),
        "output: {}",
        v
    );
    assert!(
        !h.calls().iter().any(is_grant_check),
        "grant check must be skipped on dry-run"
    );
}

// ── AC-7: sell path → grant-check gets --action sell --amount <shares> ───────

#[tokio::test]
async fn ac7_sell_grant_check_receives_sell_action_and_shares() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;
    h.mock_market_data().await;
    h.mock_rpc_order_path().await;
    h.mock_ctf_balance(30_000_000).await; // 30 shares in raw units
    h.mock_post_order().await;

    let out = h.run_plugin(
        &[
            "sell", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--shares", "30", "--autotrade-job", JOB_ID,
        ],
        Some("ok"),
        true,
        &[],
    );

    let v = stdout_json(&out);
    assert_eq!(v["ok"].as_bool(), Some(true), "sell should succeed: {}", v);
    assert_eq!(v["data"]["order_id"].as_str(), Some("test-order-12345"), "output: {}", v);
    assert_eq!(v["data"]["autotradeJob"].as_str(), Some(JOB_ID), "output: {}", v);

    let calls = h.calls();
    assert_grant_check_args(&calls, "sell", "30");
    assert!(is_grant_check(&calls[0]), "grant check must run first: {:?}", calls[0].args);
}

#[tokio::test]
async fn ac7_sell_grant_denied_fails_closed() {
    let h = Harness::new().await;
    h.mock_region_accessible().await;

    let out = h.run_plugin(
        &[
            "sell", "--market-id", TEST_CONDITION_ID, "--outcome", "yes",
            "--shares", "30", "--autotrade-job", JOB_ID,
        ],
        Some("deny"),
        false,
        &[],
    );

    assert_denied_reason_exact(&out, "per-trade cap exceeded");
    let calls = h.calls();
    assert_grant_check_args(&calls, "sell", "30");
    assert_no_auth_or_signing(&calls);
}
