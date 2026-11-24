pub mod balance;
pub mod bridge;
pub mod chains;
pub mod quickstart;
pub mod quote;
pub mod routes;
pub mod status;
pub mod tokens;

/// Render a structured error JSON for stdout output.
///
/// Per knowledge base GEN-001: every command must surface errors as JSON on stdout
/// (NOT exit non-zero, NOT stderr) so downstream agents can match on `error_code`.
pub fn error_response(msg: &str, code: &str, suggestion: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}
