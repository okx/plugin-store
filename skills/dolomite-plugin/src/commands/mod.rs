pub mod borrow;
pub mod markets;
pub mod positions;
pub mod quickstart;
pub mod repay;
pub mod supply;
pub mod withdraw;

/// Render a structured error JSON for stdout output.
/// GEN-001: every command failure must surface as JSON on stdout.
pub fn error_response(msg: &str, code: &str, suggestion: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}
