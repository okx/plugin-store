use std::process::Command;
use serde_json::Value;

/// Resolve the wallet address for the given chain_id from the onchainos CLI.
/// Uses `onchainos wallet addresses` and parses data.evm[].address matching chainIndex.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // fallback: use first EVM address
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {}", chain_id)
}

/// Execute a write operation via `onchainos wallet contract-call`.
/// Requires --confirm flag to actually broadcast; without it returns a preview.
/// In dry_run mode, returns a mock response without calling onchainos.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    value_wei: u128,
    confirm: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {"txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"},
            "calldata": input_data
        }));
    }
    if !confirm {
        return Ok(serde_json::json!({
            "preview": true,
            "message": "Run with --confirm to broadcast this transaction",
            "to": to,
            "calldata": input_data,
            "value_wei": value_wei.to_string()
        }));
    }
    let chain_str = chain_id.to_string();
    let value_str = value_wei.to_string();
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            &chain_str,
            "--to",
            to,
            "--input-data",
            input_data,
            "--amt",
            &value_str,
        ])
        .output()?;
    Ok(serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .unwrap_or_else(|_| serde_json::json!({"ok": false, "error": "failed to parse onchainos output"})))
}

/// Extract txHash from a wallet_contract_call response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
