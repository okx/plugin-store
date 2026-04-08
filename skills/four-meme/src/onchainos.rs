use std::process::Command;
use serde_json::Value;

pub fn resolve_wallet() -> anyhow::Result<String> {
    let output = Command::new("onchainos").args(["wallet", "addresses"]).output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve EVM wallet address")
}

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
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" }
        }));
    }
    if !confirm {
        return Ok(serde_json::json!({
            "preview": true,
            "message": "Run with --confirm to broadcast this transaction.",
            "to": to,
            "calldata": input_data,
            "value_wei": value_wei.to_string(),
            "chain_id": chain_id
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
        .unwrap_or_else(|_| serde_json::json!({
            "ok": false,
            "raw": String::from_utf8_lossy(&output.stdout).to_string()
        })))
}
