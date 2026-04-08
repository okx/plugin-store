use anyhow::Result;
use serde_json::Value;

use crate::config::SUI_RPC_URL;

/// Make a raw Sui JSON-RPC call.
pub async fn sui_rpc_call(method: &str, params: Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });
    let resp = client
        .post(SUI_RPC_URL)
        .json(&body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("Sui RPC error: {}", err);
    }
    Ok(resp["result"].clone())
}

/// Fetch a dynamic field object (e.g. user balance in a table).
pub async fn get_dynamic_field_object(parent_id: &str, name_type: &str, name_value: &str) -> Result<Value> {
    sui_rpc_call(
        "suix_getDynamicFieldObject",
        serde_json::json!([
            parent_id,
            {"type": name_type, "value": name_value}
        ]),
    )
    .await
}
