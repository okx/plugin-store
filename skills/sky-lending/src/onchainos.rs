use std::process::Command;
use serde_json::Value;

/// Resolve the wallet address for the given chain_id using `onchainos wallet addresses`.
/// If dry_run is true, returns the zero address immediately.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_id_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {}", chain_id)
}

/// Execute a contract-call write transaction via onchainos CLI.
/// dry_run=true: return simulated response without broadcasting.
#[allow(dead_code)]
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
pub fn eth_call(chain_id: u64, to: &str, input_data: &str) -> anyhow::Result<Value> {
    let rpc_url = match chain_id {
        1 => "https://ethereum.publicnode.com",
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
    let result_hex = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(serde_json::json!({
        "ok": true,
        "data": { "result": result_hex }
    }))
}

#[allow(dead_code)]
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
