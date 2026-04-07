use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in EVM wallet address on Ethereum mainnet (chain 1).
/// NOTE: `onchainos wallet balance --chain 1 --output json` is NOT supported on chain 1.
/// Use `onchainos wallet addresses` and filter by chainIndex == "1".
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    // Try wallet addresses first (works on all chains including chain 1)
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
        if let Some(evm_arr) = json["data"]["evm"].as_array() {
            let chain_str = chain_id.to_string();
            for entry in evm_arr {
                if entry["chainIndex"].as_str() == Some(&chain_str) {
                    if let Some(addr) = entry["address"].as_str() {
                        if !addr.is_empty() {
                            return Ok(addr.to_string());
                        }
                    }
                }
            }
            // If no chain-specific entry, take first EVM address
            if let Some(first) = evm_arr.first() {
                if let Some(addr) = first["address"].as_str() {
                    if !addr.is_empty() {
                        return Ok(addr.to_string());
                    }
                }
            }
        }
    }
    // Fallback: wallet balance without --output json
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_id.to_string()])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
        if let Some(addr) = json["data"]["details"]
            .get(0)
            .and_then(|d| d["tokenAssets"].get(0))
            .and_then(|t| t["address"].as_str())
        {
            if !addr.is_empty() {
                return Ok(addr.to_string());
            }
        }
        if let Some(addr) = json["data"]["address"].as_str() {
            if !addr.is_empty() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address. Ensure onchainos is logged in.")
}

/// Submit a transaction via onchainos wallet contract-call.
/// dry_run=true: return simulated response without calling onchainos.
/// NOTE: Never pass --dry-run to onchainos — handle it here with early return.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>, // ETH value in wei (for ETH-native calls)
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": input_data
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet",
        "contract-call",
        "--chain",
        &chain_str,
        "--to",
        to,
        "--input-data",
        input_data,
        "--force",
    ];

    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }

    let from_str_owned;
    if let Some(f) = from {
        from_str_owned = f.to_string();
        args.extend_from_slice(&["--from", &from_str_owned]);
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Read-only eth_call via direct JSON-RPC.
pub fn eth_call(chain_id: u64, to: &str, input_data: &str) -> anyhow::Result<Value> {
    let rpc_url = crate::config::RPC_URL;
    let _ = chain_id; // Currently only Ethereum mainnet supported
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": input_data },
            "latest"
        ],
        "id": 1
    });
    let client = reqwest::blocking::Client::new();
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()?
        .json()?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    let result_hex = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(serde_json::json!({
        "ok": true,
        "data": { "result": result_hex }
    }))
}

/// Extract txHash from onchainos response.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["data"]["swapTxHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
