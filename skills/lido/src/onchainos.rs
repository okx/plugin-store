use std::process::Command;
use serde_json::Value;

pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try data.address first (legacy), then data.details[0].tokenAssets[0].address
    if let Some(addr) = json["data"]["address"].as_str() {
        return Ok(addr.to_string());
    }
    if let Some(addr) = json["data"]["details"][0]["tokenAssets"][0]["address"].as_str() {
        return Ok(addr.to_string());
    }
    // Also try addresses endpoint via wallet addresses
    Ok(String::new())
}

/// dry_run=true: early return simulated response. Never pass --dry-run to onchainos.
/// force=true: pass --force to onchainos to bypass confirmation prompts and actually broadcast.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
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

/// Read-only eth_call via direct JSON-RPC to public Ethereum RPC endpoint.
/// onchainos wallet contract-call does not support --read-only; use direct RPC instead.
pub fn eth_call(chain_id: u64, to: &str, input_data: &str) -> anyhow::Result<Value> {
    let rpc_url = match chain_id {
        1 => "https://ethereum.publicnode.com",
        8453 => "https://base-rpc.publicnode.com",
        _ => anyhow::bail!("Unsupported chain_id for eth_call: {}", chain_id),
    };
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
    // Return in a shape compatible with rpc::extract_return_data
    let result_hex = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(serde_json::json!({
        "ok": true,
        "data": { "result": result_hex }
    }))
}

pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
