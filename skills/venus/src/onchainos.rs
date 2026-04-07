// Venus — onchainos CLI wrapper

use anyhow::Result;
use serde_json::Value;
use std::process::Command;

/// Resolve EVM wallet address for given chain
pub fn resolve_wallet(chain_id: u64) -> Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // fallback: data.address
    if let Some(addr) = json["data"]["address"].as_str() {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // fallback: wallet addresses command
    let out2 = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let j2: Value = serde_json::from_str(&String::from_utf8_lossy(&out2.stdout))?;
    let chain_idx = chain_id.to_string();
    if let Some(evm_list) = j2["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_idx) {
                if let Some(addr) = entry["address"].as_str() {
                    if !addr.is_empty() {
                        return Ok(addr.to_string());
                    }
                }
            }
        }
    }
    anyhow::bail!("Cannot resolve wallet address for chain {}", chain_id)
}

/// Submit a contract call via onchainos wallet contract-call
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    amt: Option<u64>,
    dry_run: bool,
) -> Result<Value> {
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
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_str,
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
    ];
    if let Some(v) = amt {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// ERC-20 approve
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    dry_run: bool,
) -> Result<Value> {
    // approve(address,uint256) selector = 0x095ea7b3
    let spender_padded = format!("{:0>64}", &spender[2..]);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, None, dry_run).await
}

/// Extract txHash from onchainos response
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
