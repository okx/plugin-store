pub mod deposit;
pub mod positions;
pub mod quickstart;
pub mod vaults;
pub mod withdraw;

/// Format a business error as a structured JSON string for stdout.
pub fn error_response(err: &anyhow::Error, vault: Option<&str>) -> String {
    let msg = format!("{:#}", err);
    let (error_code, suggestion) = classify_error(&msg, vault);
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": error_code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}

fn classify_error(msg: &str, vault: Option<&str>) -> (&'static str, String) {
    let vault_hint = vault.unwrap_or("this vault");
    if msg.contains("WALLET_NOT_FOUND") || msg.contains("resolve wallet") || msg.contains("resolve Solana wallet") {
        return ("WALLET_NOT_FOUND",
            "Run `onchainos wallet balance --chain 501` to verify login, or pass --wallet <address>.".into());
    }
    if msg.contains("base64") || msg.contains("base58") {
        return ("TX_ENCODING_ERROR",
            "Transaction encoding failed. The API transaction may have expired — retry.".into());
    }
    if msg.contains("contract-call failed") || msg.contains("contract-call returned error") {
        return ("TX_BROADCAST_ERROR",
            format!("Transaction broadcast failed for {}. Check wallet balance and try again.", vault_hint));
    }
    if msg.contains("Timeout") || msg.contains("timeout") {
        return ("TX_TIMEOUT",
            "Transaction did not confirm within 60s. Check Solscan for the tx hash.".into());
    }
    if msg.contains("failed on-chain") {
        return ("TX_FAILED_ON_CHAIN",
            "Transaction was included in a block but execution failed. See failReason for details.".into());
    }
    if msg.contains("insufficient") || msg.contains("Insufficient") {
        return ("INSUFFICIENT_BALANCE",
            format!("Insufficient token balance to deposit into {}.", vault_hint));
    }
    if msg.contains("fewer than 32 characters") || msg.contains("kvault must") || msg.contains("vault must") {
        return ("INVALID_VAULT_ADDRESS",
            format!("'{}' is not a valid vault address. Use `kamino-liquidity vaults` to list valid vault addresses.", vault_hint));
    }
    if msg.contains("Kamino API") {
        return ("KAMINO_API_ERROR",
            "Kamino API returned an error. Check vault address and amount, then retry.".into());
    }
    ("UNKNOWN_ERROR", "See error field for details.".into())
}
