// Acceptance tests for the autotrade (copy-trading) authorization gate.
//
// The load-bearing property under test is that nothing is signed or submitted unless
// onchainos explicitly authorizes the order. Each test therefore asserts on the
// mock's invocation log, not just on the plugin's exit status or stdout.

mod common;

use common::*;

fn order_args<'a>(job: Option<&'a str>, size: &'a str) -> Vec<&'a str> {
    let mut v = vec![
        "order",
        "--coin",
        "BTC",
        "--side",
        "buy",
        "--size",
        size,
        "--type",
        "market",
        "--confirm",
    ];
    if let Some(j) = job {
        v.push("--autotrade-job");
        v.push(j);
    }
    v
}

fn assert_no_signing(log: &CallLog) {
    let signing = log.signing_calls();
    assert!(
        signing.is_empty(),
        "expected zero signing calls, got: {:?}",
        signing
    );
}

fn assert_rejected(run: &Run, code: &str) {
    let v = run.json();
    assert_eq!(
        v["ok"].as_bool(),
        Some(false),
        "expected a rejection, stdout was: {}",
        run.stdout()
    );
    assert_eq!(
        v["error_code"].as_str(),
        Some(code),
        "unexpected error_code, stdout was: {}",
        run.stdout()
    );
}

// no flag ⇒ no grant check at all, and the order proceeds as before.
#[test]
fn without_flag_no_grant_check() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t01");
    let run = run_plugin(&order_args(None, "0.01"), &api, &log, None, &[]);
    assert!(
        log.grant_checks().is_empty(),
        "grant check must not run without --autotrade-job: {:?}",
        log.grant_checks()
    );
    assert_eq!(run.json()["ok"].as_bool(), Some(true), "{}", run.stdout());
}

// grant allows ⇒ the order reaches signing.
#[test]
fn grant_ok_proceeds_to_signing() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t02");
    let run = run_plugin(
        &order_args(Some("job-ac02"), "0.01"),
        &api,
        &log,
        Some("ok"),
        &[],
    );
    assert_eq!(
        log.grant_checks().len(),
        1,
        "expected exactly one grant check"
    );
    assert!(
        !log.signing_calls().is_empty(),
        "grant allowed but nothing was signed; stdout: {}",
        run.stdout()
    );
    assert_eq!(run.json()["ok"].as_bool(), Some(true), "{}", run.stdout());
}

// explicit denial ⇒ reason passed through verbatim, zero signing.
#[test]
fn grant_deny_blocks_and_passes_reason() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t03");
    let run = run_plugin(
        &order_args(Some("job-ac03"), "0.01"),
        &api,
        &log,
        Some("deny"),
        &[("MOCK_ONCHAINOS_GRANT_REASON", "per-trade cap exceeded")],
    );
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    assert!(
        run.json()["error"]
            .as_str()
            .unwrap_or_default()
            .contains("per-trade cap exceeded"),
        "reason not surfaced: {}",
        run.stdout()
    );
    assert_no_signing(&log);
}

// a hung grant check is abandoned after the timeout and refused.
#[test]
fn grant_timeout_refuses() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t04");
    let run = run_plugin(
        &order_args(Some("job-ac04"), "0.01"),
        &api,
        &log,
        Some("hang"),
        &[
            ("HYPERLIQUID_GRANT_CHECK_TIMEOUT_SECS", "1"),
            ("MOCK_ONCHAINOS_GRANT_HANG_SECS", "30"),
        ],
    );
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    assert!(
        run.json()["error"]
            .as_str()
            .unwrap_or_default()
            .contains("timeout"),
        "expected a timeout reason: {}",
        run.stdout()
    );
    assert_no_signing(&log);
}

// unparseable grant output is a refusal, never an implicit pass.
#[test]
fn grant_invalid_json_refuses() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t05");
    let run = run_plugin(
        &order_args(Some("job-ac05"), "0.01"),
        &api,
        &log,
        Some("badjson"),
        &[],
    );
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    assert!(
        run.json()["error"]
            .as_str()
            .unwrap_or_default()
            .contains("invalid JSON"),
        "expected an invalid-JSON reason: {}",
        run.stdout()
    );
    assert_no_signing(&log);
}

// an onchainos predating the subcommand yields an upgrade hint, not a pass.
#[test]
fn old_onchainos_refuses_with_upgrade_hint() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t06");
    // No MOCK_ONCHAINOS_GRANT_RESULT ⇒ the fixture reports an unrecognized subcommand.
    let run = run_plugin(&order_args(Some("job-ac06"), "0.01"), &api, &log, None, &[]);
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    assert!(
        run.json()["error"]
            .as_str()
            .unwrap_or_default()
            .contains("upgrade"),
        "expected an upgrade hint: {}",
        run.stdout()
    );
    assert_no_signing(&log);
}

// an unlaunchable onchainos is a refusal with an install hint.
#[test]
fn missing_onchainos_refuses() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t07");
    let run = run_plugin(
        &order_args(Some("job-ac07"), "0.01"),
        &api,
        &log,
        Some("ok"),
        &[(
            "HYPERLIQUID_ONCHAINOS_BIN",
            "/nonexistent/path/to/onchainos-binary",
        )],
    );
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    let err = run.json()["error"].as_str().unwrap_or_default().to_string();
    assert!(
        err.contains("installed") || err.contains("PATH") || err.contains("launch"),
        "expected an install/launch hint: {}",
        run.stdout()
    );
}

// a malformed job ID costs nothing: no subprocess, no network, no signing.
#[test]
fn invalid_job_id_spawns_nothing() {
    let api = start_mock_api(MockApiConfig::default());
    for bad in [
        "job id with spaces",
        "job;rm -rf /",
        "",
        &"x".repeat(129),
        "job$(whoami)",
    ] {
        let log = CallLog::new("t08");
        let run = run_plugin(&order_args(Some(bad), "0.01"), &api, &log, Some("ok"), &[]);
        assert_rejected(&run, "INVALID_INPUT");
        assert!(
            log.lines().is_empty(),
            "job id {:?} must not spawn onchainos at all, got: {:?}",
            bad,
            log.lines()
        );
    }
}

// the accepted charset really is accepted.
#[test]
fn valid_job_ids_reach_the_grant_check() {
    let api = start_mock_api(MockApiConfig::default());
    for good in ["job-123", "JOB_456", "0xdeadbeef", "a", &"y".repeat(128)] {
        let log = CallLog::new("t08ok");
        let run = run_plugin(&order_args(Some(good), "0.01"), &api, &log, Some("ok"), &[]);
        assert_eq!(
            log.grant_checks().len(),
            1,
            "job id {:?} should have reached the grant check; calls={:?}; stdout: {}",
            good,
            log.lines(),
            run.stdout()
        );
    }
}

// dry-run signs nothing, so it skips the check and says so.
#[test]
fn dry_run_skips_grant_check_and_marks_output() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t09");
    let mut args = order_args(Some("job-ac09"), "0.01");
    args.push("--dry-run");
    let run = run_plugin(&args, &api, &log, Some("ok"), &[]);
    assert!(
        log.grant_checks().is_empty(),
        "dry-run must not call the grant check: {:?}",
        log.grant_checks()
    );
    let preview = run.first_json();
    assert_eq!(
        preview["preview"]["autotradeGrantCheck"].as_str(),
        Some("skipped (dry-run)"),
        "dry-run output must say the check was skipped: {}",
        run.stdout()
    );
    assert_no_signing(&log);
}

// an authorized, successful order echoes the job ID back.
#[test]
fn success_output_carries_job_id() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t10");
    let run = run_plugin(
        &order_args(Some("job-ac10"), "0.01"),
        &api,
        &log,
        Some("ok"),
        &[],
    );
    assert_eq!(
        run.json()["autotradeJob"].as_str(),
        Some("job-ac10"),
        "{}",
        run.stdout()
    );
}

// the leverage update is its own signed action, so a denial must precede it.
#[test]
fn denied_order_does_not_sign_leverage_update() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t11");
    let mut args = order_args(Some("job-ac11"), "0.01");
    args.push("--leverage");
    args.push("10");
    let run = run_plugin(&args, &api, &log, Some("deny"), &[]);
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    assert_no_signing(&log);
}

// a crashing onchainos must not leak the grant file it was reading.
#[test]
fn no_grant_material_in_output() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("t12");
    let run = run_plugin(
        &order_args(Some("job-ac12"), "0.01"),
        &api,
        &log,
        Some("crash"),
        &[],
    );
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    let combined = format!("{}{}", run.stdout(), run.stderr());
    assert!(
        !combined.contains(GRANT_FILE_MARKER),
        "grant file path leaked into plugin output: {}",
        combined
    );
    assert_no_signing(&log);
}

// below the exchange minimum, the authorized path refuses instead of resizing,
// while the interactive path keeps bumping. The two must not converge.
#[test]
fn min_notional_refused_on_autotrade_path_only() {
    let api = start_mock_api(MockApiConfig::default());

    // $100k mid × 0.00001 = $1 notional, below the $10 minimum.
    let log_auto = CallLog::new("t14auto");
    let auto = run_plugin(
        &order_args(Some("job-ac14"), "0.00001"),
        &api,
        &log_auto,
        Some("ok"),
        &[],
    );
    assert_rejected(&auto, "ORDER_BELOW_MIN_NOTIONAL");
    assert_no_signing(&log_auto);
    assert!(
        log_auto.grant_checks().is_empty(),
        "the size refusal precedes the grant check, so none should have run: {:?}",
        log_auto.grant_checks()
    );

    let log_manual = CallLog::new("t14manual");
    let manual = run_plugin(&order_args(None, "0.00001"), &api, &log_manual, None, &[]);
    assert_eq!(
        manual.json()["ok"].as_bool(),
        Some(true),
        "interactive path must still auto-bump: {}",
        manual.stdout()
    );
    assert!(
        manual.stderr().contains("[auto-adjust]"),
        "expected the auto-adjust notice on the interactive path: {}",
        manual.stderr()
    );
}

#[test]
fn off_grid_autotrade_size_is_rejected_instead_of_rounded() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("off-grid");
    let run = run_plugin(
        &order_args(Some("job-off-grid"), "0.010001"),
        &api,
        &log,
        Some("ok"),
        &[],
    );
    assert_rejected(&run, "INVALID_INPUT");
    assert!(
        log.grant_checks().is_empty(),
        "off-grid card must be rejected before authorization: {:?}",
        log.grant_checks()
    );
    assert_no_signing(&log);
    assert!(
        run.stdout()
            .contains("Do not fund, modify, or retry this card"),
        "{}",
        run.stdout()
    );
}

#[test]
fn off_grid_autotrade_limit_price_is_rejected_instead_of_rounded() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("off-grid-price");
    let mut args = order_args(Some("job-off-grid-price"), "0.01");
    let order_type = args
        .iter()
        .position(|arg| *arg == "market")
        .expect("market order type");
    args[order_type] = "limit";
    args.extend(["--price", "100000.1"]);
    let run = run_plugin(&args, &api, &log, Some("ok"), &[]);
    assert_rejected(&run, "INVALID_INPUT");
    assert!(log.grant_checks().is_empty());
    assert_no_signing(&log);
}

#[test]
fn off_grid_autotrade_trigger_price_is_rejected_instead_of_rounded() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("off-grid-trigger");
    let mut args = order_args(Some("job-off-grid-trigger"), "0.01");
    args.extend(["--sl-px", "95000.1"]);
    let run = run_plugin(&args, &api, &log, Some("ok"), &[]);
    assert_rejected(&run, "INVALID_INPUT");
    assert!(log.grant_checks().is_empty());
    assert_no_signing(&log);
}

#[test]
fn invalid_autotrade_slippage_is_rejected_before_network_or_signing() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("invalid-slippage");
    let mut args = order_args(Some("job-invalid-slippage"), "0.01");
    args.extend(["--slippage", "100"]);
    let run = run_plugin(&args, &api, &log, Some("ok"), &[]);
    assert_rejected(&run, "INVALID_INPUT");
    assert!(log.lines().is_empty(), "{:?}", log.lines());
    assert_no_signing(&log);
}

#[test]
fn only_isolated_autotrade_requires_explicit_flag() {
    let api = start_mock_api(MockApiConfig {
        only_isolated: true,
        ..MockApiConfig::default()
    });
    let log = CallLog::new("isolated-missing");
    let mut args = order_args(Some("job-isolated"), "0.01");
    args.extend(["--leverage", "10"]);
    let run = run_plugin(&args, &api, &log, Some("ok"), &[]);
    assert_rejected(&run, "INVALID_INPUT");
    assert!(
        log.grant_checks().is_empty(),
        "missing --isolated must fail before authorization: {:?}",
        log.grant_checks()
    );
    assert_no_signing(&log);
}

#[test]
fn only_isolated_autotrade_accepts_explicit_flag() {
    let api = start_mock_api(MockApiConfig {
        only_isolated: true,
        ..MockApiConfig::default()
    });
    let log = CallLog::new("isolated-explicit");
    let mut args = order_args(Some("job-isolated-ok"), "0.01");
    args.extend(["--leverage", "10", "--isolated"]);
    let run = run_plugin(&args, &api, &log, Some("ok"), &[]);
    assert_eq!(run.json()["ok"].as_bool(), Some(true), "{}", run.stdout());
    assert_eq!(log.grant_checks().len(), 1);
    assert_eq!(
        log.signing_calls().len(),
        2,
        "explicit isolated card should sign leverage update and order"
    );
}

#[test]
fn autotrade_balance_failure_does_not_suggest_funding_or_retry() {
    let api = start_mock_api(MockApiConfig {
        perp_withdrawable: "0".to_string(),
        ..MockApiConfig::default()
    });
    let log = CallLog::new("unfunded");
    let run = run_plugin(
        &order_args(Some("job-unfunded"), "0.01"),
        &api,
        &log,
        Some("ok"),
        &[],
    );
    assert_rejected(&run, "PERP_INSUFFICIENT_BALANCE");
    let output = run.stdout();
    assert!(
        output.contains("Do not fund, modify, or retry this card"),
        "{}",
        output
    );
    assert!(!output.contains("transfer --amount"), "{}", output);
    assert!(!output.contains("deposit --amount"), "{}", output);
    assert!(
        log.grant_checks().is_empty(),
        "unexecutable card must fail before authorization"
    );
    assert_no_signing(&log);
}

// The gate must submit the amount that actually gets broadcast, and the venue/action
// the contract specifies.
#[test]
fn grant_check_arguments_match_the_contract() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("args");
    let _ = run_plugin(
        &order_args(Some("job-args"), "0.01"),
        &api,
        &log,
        Some("ok"),
        &[],
    );
    let checks = log.grant_checks();
    assert_eq!(checks.len(), 1);
    let call = &checks[0];
    for expected in [
        "\"--venue\",\"hyperliquid\"",
        "\"--action\",\"buy\"",
        "\"--amount\",\"1050.00\"",
        "\"--job-id\",\"job-args\"",
        "\"--format\",\"json\"",
    ] {
        assert!(
            call.replace(' ', "").contains(expected),
            "grant check call missing {}: {}",
            expected,
            call
        );
    }
}

// close: the submitted amount is the resolved position size even when --size is omitted,
// and the action is the closing direction rather than the position's own side.
#[test]
fn close_gate_submits_resolved_size_and_closing_side() {
    let api = start_mock_api(MockApiConfig {
        position_szi: Some("0.25".to_string()),
        ..MockApiConfig::default()
    });
    let log = CallLog::new("close-ok");
    let run = run_plugin(
        &[
            "close",
            "--coin",
            "BTC",
            "--confirm",
            "--autotrade-job",
            "job-close",
        ],
        &api,
        &log,
        Some("ok"),
        &[],
    );
    let checks = log.grant_checks();
    assert_eq!(checks.len(), 1, "stdout: {}", run.stdout());
    let call = checks[0].replace(' ', "");
    assert!(
        call.contains("\"--amount\",\"25000.00\""),
        "close must submit the resolved size as a quote-currency notional: {}",
        checks[0]
    );
    // Closing a long is a sell.
    assert!(
        call.contains("\"--action\",\"sell\""),
        "close must submit the closing direction: {}",
        checks[0]
    );
    assert_eq!(
        run.json()["autotradeJob"].as_str(),
        Some("job-close"),
        "{}",
        run.stdout()
    );
}

// close: a denial blocks the close, same as an order.
#[test]
fn close_denied_does_not_sign() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("close-deny");
    let run = run_plugin(
        &[
            "close",
            "--coin",
            "BTC",
            "--confirm",
            "--autotrade-job",
            "job-close-deny",
        ],
        &api,
        &log,
        Some("deny"),
        &[],
    );
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    assert_no_signing(&log);
}

// A preview (no --confirm) is not an execution, so it must not consume an authorization.
#[test]
fn preview_without_confirm_does_not_call_grant_check() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("preview");
    let mut args = order_args(Some("job-preview"), "0.01");
    args.retain(|a| *a != "--confirm");
    let run = run_plugin(&args, &api, &log, Some("ok"), &[]);
    assert!(
        log.grant_checks().is_empty(),
        "preview must not consume an authorization: {:?}",
        log.grant_checks()
    );
    assert!(
        run.stderr().contains("[PREVIEW]"),
        "expected the preview notice: {}",
        run.stderr()
    );
    assert_no_signing(&log);
}

// The submitted amount must be the quote-currency notional, not the base size: the written
// cap is denominated in quote stablecoin, so a base-unit figure would clear any cap.
#[test]
fn submitted_amount_is_quote_notional_not_base_size() {
    let api = start_mock_api(MockApiConfig::default());
    let log = CallLog::new("quote");
    let size = "0.01";
    let _ = run_plugin(
        &order_args(Some("job-quote"), size),
        &api,
        &log,
        Some("ok"),
        &[],
    );
    let call = log.grant_checks()[0].replace(' ', "");

    let amount = call
        .split("\"--amount\",\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("amount present")
        .parse::<f64>()
        .expect("amount is numeric");

    let base: f64 = size.parse().unwrap();
    let mid: f64 = TEST_MID.parse().unwrap();
    assert!(
        amount >= base * mid,
        "amount {} must be at least size x mid ({}), i.e. quote-denominated",
        amount,
        base * mid
    );
    assert!(
        amount > base * 100.0,
        "amount {} looks like a base-unit size, not a notional",
        amount
    );
}

// With no usable price the notional cannot be established, so the order is refused rather
// than submitted with a figure that would clear any cap.
#[test]
fn missing_price_refuses_instead_of_submitting_zero() {
    let api = start_mock_api(MockApiConfig {
        mid: "0".to_string(),
        ..MockApiConfig::default()
    });
    let log = CallLog::new("nomid");
    let run = run_plugin(
        &order_args(Some("job-nomid"), "0.01"),
        &api,
        &log,
        Some("ok"),
        &[],
    );
    assert_rejected(&run, "AUTOTRADE_GRANT_DENIED");
    assert!(
        log.grant_checks().is_empty(),
        "no price ⇒ nothing should be submitted for authorization: {:?}",
        log.grant_checks()
    );
    assert_no_signing(&log);
}
