use anyhow::Context;
use serde::Deserialize;

pub const CHAIN_ETH: u64 = 1;
pub const CHAIN_ARB: u64 = 42161;

#[derive(Debug, Deserialize)]
pub struct EthLog {
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct EthTransaction {
    pub to: Option<String>,
}

pub fn rpc_url(chain_id: u64) -> anyhow::Result<&'static str> {
    match chain_id {
        1     => Ok("https://ethereum.publicnode.com"),
        42161 => Ok("https://arb1.arbitrum.io/rpc"),
        _     => anyhow::bail!("Unsupported chain ID {}. Use 1 (Ethereum) or 42161 (Arbitrum).", chain_id),
    }
}

pub fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        1     => "Ethereum",
        42161 => "Arbitrum",
        _     => "Unknown",
    }
}

/// Make a raw eth_call and return the result hex string (no 0x prefix).
pub async fn eth_call(chain_id: u64, to: &str, data: &str) -> anyhow::Result<String> {
    let rpc = rpc_url(chain_id)?;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "id": 1,
        "params": [{"to": to, "data": data}, "latest"]
    });
    let resp: serde_json::Value = client
        .post(rpc)
        .json(&body)
        .send().await
        .context("RPC request failed")?
        .json().await
        .context("Failed to parse RPC response")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("RPC error: {}", err);
    }
    let result = resp.get("result")
        .and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("No result in RPC response"))?;
    Ok(result.trim_start_matches("0x").to_string())
}

/// eth_getLogs filtered by address and up to 4 topics (None = wildcard).
pub async fn eth_get_logs(
    chain_id: u64,
    address: &str,
    topics: &[Option<&str>],
) -> anyhow::Result<Vec<EthLog>> {
    let rpc = rpc_url(chain_id)?;
    let client = reqwest::Client::new();
    let topic_arr: Vec<serde_json::Value> = topics.iter().map(|t| match t {
        Some(s) => serde_json::Value::String(s.to_string()),
        None    => serde_json::Value::Null,
    }).collect();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getLogs",
        "id": 1,
        "params": [{"address": address, "fromBlock": "0x0", "toBlock": "latest", "topics": topic_arr}]
    });
    let resp: serde_json::Value = client
        .post(rpc)
        .json(&body)
        .send().await
        .context("eth_getLogs request failed")?
        .json().await
        .context("Failed to parse eth_getLogs response")?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("RPC error: {}", err);
    }
    let logs: Vec<EthLog> = serde_json::from_value(
        resp.get("result").cloned().unwrap_or(serde_json::Value::Array(vec![]))
    ).unwrap_or_default();
    Ok(logs)
}

/// eth_getTransactionByHash — returns the transaction's `to` field.
pub async fn eth_get_transaction(chain_id: u64, tx_hash: &str) -> anyhow::Result<EthTransaction> {
    let rpc = rpc_url(chain_id)?;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionByHash",
        "id": 1,
        "params": [tx_hash]
    });
    let resp: serde_json::Value = client
        .post(rpc)
        .json(&body)
        .send().await
        .context("eth_getTransactionByHash request failed")?
        .json().await
        .context("Failed to parse eth_getTransactionByHash response")?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("RPC error: {}", err);
    }
    let tx: EthTransaction = serde_json::from_value(
        resp.get("result").cloned().unwrap_or(serde_json::Value::Null)
    ).context("Could not parse transaction")?;
    Ok(tx)
}

/// Simulate a call via eth_call. Returns Ok(hex_result) or Err(revert_message).
pub async fn eth_call_simulate(chain_id: u64, to: &str, data: &str, from: &str) -> Result<String, String> {
    let rpc = match rpc_url(chain_id) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "id": 1,
        "params": [{"to": to, "data": data, "from": from}, "latest"]
    });
    let resp: serde_json::Value = match client.post(rpc).json(&body).send().await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        },
        Err(e) => return Err(e.to_string()),
    };
    if let Some(err) = resp.get("error") {
        return Err(err.to_string());
    }
    Ok(resp.get("result").and_then(|r| r.as_str()).unwrap_or("").to_string())
}
