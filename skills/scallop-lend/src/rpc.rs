use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::config::SUI_RPC_URL;

/// Send a Sui JSON-RPC request
pub async fn sui_rpc(client: &Client, method: &str, params: Value) -> Result<Value> {
    let body = json!({
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

    if let Some(error) = resp.get("error") {
        return Err(anyhow!("Sui RPC error: {}", error));
    }

    Ok(resp["result"].clone())
}

/// Fetch a Move object with content
pub async fn get_object(client: &Client, object_id: &str) -> Result<Value> {
    let result = sui_rpc(
        client,
        "sui_getObject",
        json!([
            object_id,
            {
                "showContent": true,
                "showType": true,
                "showOwner": false,
                "showDisplay": false
            }
        ]),
    )
    .await?;

    if result.get("error").is_some() {
        return Err(anyhow!("Object not found: {}", object_id));
    }

    Ok(result)
}

/// Get all balances for an address
pub async fn get_all_balances(client: &Client, address: &str) -> Result<Value> {
    sui_rpc(client, "suix_getAllBalances", json!([address])).await
}

/// Get objects owned by address filtered by type
pub async fn get_owned_objects(
    client: &Client,
    address: &str,
    type_filter: Option<&str>,
) -> Result<Value> {
    let filter = if let Some(t) = type_filter {
        json!({
            "filter": { "StructType": t },
            "options": { "showContent": true, "showType": true }
        })
    } else {
        json!({ "options": { "showContent": true, "showType": true } })
    };

    sui_rpc(
        client,
        "suix_getOwnedObjects",
        json!([address, filter, null, 50]),
    )
    .await
}
