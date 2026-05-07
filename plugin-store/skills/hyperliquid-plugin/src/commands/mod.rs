pub mod address;
pub mod cancel;
pub mod cancel_batch;
pub mod close;
pub mod deposit;
pub mod dex_list;
pub mod dex_transfer;
pub mod markets;
pub mod outcome_list;
pub mod outcome_positions;
pub mod usdh_fund;
pub mod outcome_buy;
pub mod outcome_sell;
pub mod outcome_cancel;
pub mod abstraction;
pub mod evm_send;
pub mod get_gas;
pub mod order;
pub mod order_batch;
pub mod orders;
pub mod positions;
pub mod prices;
pub mod register;
pub mod spot_balances;
pub mod spot_cancel;
pub mod spot_order;
pub mod spot_prices;
pub mod tpsl;
pub mod transfer;
pub mod withdraw;
pub mod quickstart;

/// Render a structured error JSON string for stdout output.
/// All command failures must use this instead of anyhow::bail! or ?.
pub fn error_response(msg: &str, code: &str, suggestion: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "ok": false,
        "error": msg,
        "error_code": code,
        "suggestion": suggestion,
    }))
    .unwrap_or_else(|_| format!(r#"{{"ok":false,"error":{:?}}}"#, msg))
}
