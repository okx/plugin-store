pub mod quickstart;
pub mod list_vaults;
pub mod get_vault;
pub mod positions;
pub mod health_factor;
// write commands
pub mod supply;
pub mod withdraw;
pub mod borrow;
pub mod repay;
pub mod enable_collateral;
pub mod disable_collateral;
pub mod enable_controller;
pub mod disable_controller;
pub mod claim_rewards;

/// Build a structured error JSON for stdout output (per GEN-001).
///
/// Use when a command hits a business-logic failure — the caller should `println!`
/// this and `return Ok(())` so external Agents can parse the error instead of
/// seeing only exit code 1 + stderr.
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

fn classify_error(msg: &str, ctx: Option<&str>) -> (&'static str, String) {
    let m = msg.to_lowercase();

    // Network / RPC
    if m.contains("rpc request failed") || m.contains("euler /api") {
        return (
            "API_UNAVAILABLE",
            "Euler API or chain RPC was unreachable. Wait a few seconds and retry. \
             If it persists, check that app.euler.finance and your chain's public RPC are accessible.".into(),
        );
    }
    if m.contains("error sending request") || m.contains("connection refused") || m.contains("certificate") {
        return (
            "NETWORK_UNREACHABLE",
            "Network request failed. Check internet connectivity and that app.euler.finance is reachable from your IP.".into(),
        );
    }

    // Wallet
    if m.contains("could not determine wallet address") || m.contains("wallet addresses") {
        return (
            "NO_WALLET",
            "No active onchainos wallet found. Run `onchainos wallet status` to inspect, or `onchainos wallet add` to create one.".into(),
        );
    }

    // Chain support
    if m.contains("chain") && (m.contains("not supported") || m.contains("not found in euler")) {
        return (
            "CHAIN_NOT_SUPPORTED",
            "Euler v2 plugin v0.1 supports chains: 1 (Ethereum), 8453 (Base), 42161 (Arbitrum). Pass `--chain <id>` from this list.".into(),
        );
    }

    // Tx lifecycle
    if m.contains("not confirmed within") || m.contains("execution reverted") {
        return (
            "TX_FAILED",
            "Transaction did not confirm successfully. Check the chain's block explorer for the tx hash. \
             For Euler v2 borrows, ensure a controller vault is enabled before borrowing (`enable-controller`).".into(),
        );
    }

    // Per-command fallback
    let default_code: &'static str = match ctx {
        Some("quickstart")          => "QUICKSTART_FAILED",
        Some("list-vaults")         => "LIST_VAULTS_FAILED",
        Some("get-vault")           => "GET_VAULT_FAILED",
        Some("positions")           => "POSITIONS_FAILED",
        Some("health-factor")       => "HEALTH_FACTOR_FAILED",
        Some("supply")              => "SUPPLY_FAILED",
        Some("withdraw")            => "WITHDRAW_FAILED",
        Some("borrow")              => "BORROW_FAILED",
        Some("repay")               => "REPAY_FAILED",
        Some("enable-collateral")   => "ENABLE_COLLATERAL_FAILED",
        Some("disable-collateral")  => "DISABLE_COLLATERAL_FAILED",
        Some("enable-controller")   => "ENABLE_CONTROLLER_FAILED",
        Some("disable-controller")  => "DISABLE_CONTROLLER_FAILED",
        Some("claim-rewards")       => "CLAIM_REWARDS_FAILED",
        _                           => "UNKNOWN_ERROR",
    };
    (default_code, "See error field for details. Retry the command, or run with --dry-run to inspect parameters.".into())
}
