pub mod borrow;
pub mod markets;
pub mod positions;
pub mod quickstart;
pub mod repay;
pub mod reserves;
pub mod supply;
pub mod withdraw;

/// Format any error as a structured JSON error response suitable for external Agent consumption.
/// Always prints to stdout so callers can parse it regardless of exit code.
pub fn error_response(err: &anyhow::Error, token: Option<&str>) -> String {
    let msg = format!("{:#}", err);
    let (error_code, suggestion) = classify_error(&msg, token);
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": error_code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}

fn classify_error(msg: &str, token: Option<&str>) -> (&'static str, String) {
    if msg.contains("Cannot borrow above borrow limit") {
        let tok = token.unwrap_or("this token");
        return (
            "BORROW_LIMIT_EXCEEDED",
            format!(
                "{} borrow cap is full. Try a different token such as mSOL, JitoSOL, or USDC.",
                tok
            ),
        );
    }
    if msg.contains("Net value remaining too small") {
        return (
            "REPAY_DUST_ERROR",
            "Interest accrued between query and execution — use `--amount all` to repay the full outstanding balance.".to_string(),
        );
    }
    if msg.contains("Custom:1") {
        return (
            "INSUFFICIENT_BALANCE",
            "Wallet SPL token balance is less than the required amount (often 1 atom short due to accrued interest). Use `--amount all` when repaying, or top up wallet balance.".to_string(),
        );
    }
    if msg.contains("obligation does not exist") || msg.contains("No obligation") {
        return (
            "NO_OBLIGATION",
            "No active Kamino obligation found. Supply an asset first to create an obligation account, then retry.".to_string(),
        );
    }
    if msg.contains("health factor") || msg.contains("unhealthy") {
        return (
            "HEALTH_FACTOR_TOO_LOW",
            "Withdrawing this amount would push your health factor below 1.0. Repay outstanding borrows first or reduce the withdrawal amount.".to_string(),
        );
    }
    if msg.contains("base64") || msg.contains("base58") || msg.contains("conversion failed") {
        return (
            "TX_BUILD_ERROR",
            "Transaction encoding failed. Retry — the Kamino API transaction may have expired (Solana blockhash valid ~60s).".to_string(),
        );
    }
    if msg.contains("Cannot resolve wallet") || msg.contains("wallet address") {
        return (
            "WALLET_NOT_FOUND",
            "Wallet address could not be resolved. Run `onchainos wallet balance --chain 501` to verify login, or pass `--wallet <address>`.".to_string(),
        );
    }
    if msg.contains("Insufficient") || msg.contains("insufficient") {
        return (
            "INSUFFICIENT_FUNDS",
            "Wallet balance is too low for this operation. Check balance and try a smaller amount.".to_string(),
        );
    }
    (
        "UNKNOWN_ERROR",
        "See the error field for details. If this persists, check onchainos login status and Kamino market availability.".to_string(),
    )
}
