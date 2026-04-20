pub mod balance;
pub mod buy;
pub mod withdraw;
pub mod cancel;
pub mod check_access;
pub mod deposit;
pub mod get_market;
pub mod get_positions;
pub mod get_series;
pub mod list_5m;
pub mod list_markets;
pub mod redeem;
pub mod sell;
pub mod setup_proxy;
pub mod switch_mode;

/// Build a structured error JSON string for stdout output (per GEN-001).
///
/// Use when a command hits a business-logic failure (insufficient gas, tx never
/// broadcast, revert, missing positions, etc.) — the caller should `println!` this
/// and `return Ok(())` so external agents can parse the error instead of seeing
/// only exit code 1 + stderr.
///
/// `extra_hint`, when present, is appended to `suggestion` — useful for attaching
/// context the classifier cannot derive from the error message alone (e.g. a proxy
/// wallet address discovered on-chain for this specific EOA).
pub fn error_response(
    err: &anyhow::Error,
    context: Option<&str>,
    extra_hint: Option<&str>,
) -> String {
    let msg = format!("{:#}", err);
    let (error_code, mut suggestion) = classify_error(&msg, context);
    if let Some(h) = extra_hint {
        let h = h.trim();
        if !h.is_empty() {
            suggestion.push(' ');
            suggestion.push_str(h);
        }
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": error_code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}

fn classify_error(msg: &str, _ctx: Option<&str>) -> (&'static str, String) {
    let m = msg.to_lowercase();

    if m.contains("insufficient pol") {
        return (
            "INSUFFICIENT_POL_GAS",
            "Top up POL on your EOA wallet (Polygon). Redeem costs ~0.015 POL per market; \
             batch redeem N markets needs ~N × 0.015 POL.".into(),
        );
    }
    if m.contains("no redeemable positions") {
        return (
            "NO_REDEEMABLE_POSITIONS",
            "Data API shows no redeemable positions on either the EOA or the proxy wallet. \
             If you traded in POLY_PROXY mode, run `setup-proxy` first so the plugin knows \
             your proxy address; otherwise verify your trading mode with `balance`.".into(),
        );
    }
    if m.contains("simulation reverted") || m.contains("eth_call reverted") {
        return (
            "SIMULATION_REVERTED",
            "eth_call simulation reverted before broadcast. The most common cause for redeem \
             is that the EOA does not hold the winning outcome tokens (they live in the proxy \
             wallet). Run `setup-proxy` or check your trading mode.".into(),
        );
    }
    if m.contains("not observed on-chain") || m.contains("not confirmed within") {
        return (
            "TX_NOT_CONFIRMED",
            "Tx hash was returned but never appeared on-chain. Usually means onchainos signed \
             a tx that would revert and dropped it silently. Check Polygonscan for the hash — \
             if missing, re-run with --dry-run to inspect, or verify your trading mode.".into(),
        );
    }
    if m.contains("mined but reverted") || m.contains("status 0x0") {
        return (
            "TX_REVERTED",
            "Tx mined but reverted on-chain. For redeem this usually means the calling wallet \
             does not hold the winning outcome tokens.".into(),
        );
    }
    if m.contains("neg_risk") {
        return (
            "NEG_RISK_NOT_SUPPORTED",
            "Multi-outcome (neg_risk) markets cannot be redeemed via this plugin — \
             use the Polymarket web UI.".into(),
        );
    }

    ("REDEEM_FAILED", "See error field for details.".into())
}
