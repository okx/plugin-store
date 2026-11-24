pub mod borrow;
pub mod claim_rewards;
pub mod markets;
pub mod positions;
pub mod quickstart;
pub mod repay;
pub mod supply;
pub mod swap_borrow_rate_mode;
pub mod withdraw;

/// Standardized GEN-001 error JSON. Always print to stdout; never stderr.
pub fn error_response(msg: &str, code: &str, suggestion: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?},"error_code":{:?}}}"#, msg, code))
}
