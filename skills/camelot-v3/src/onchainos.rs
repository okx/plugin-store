use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in wallet address for the given EVM chain.
/// Uses `onchainos wallet addresses` and matches by chainIndex.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_index = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try data.evm[] array, match chainIndex
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_index) {
                let addr = entry["address"].as_str().unwrap_or("").to_string();
                if !addr.is_empty() {
                    return Ok(addr);
                }
            }
        }
        // Fallback: return first EVM address
        if let Some(first) = evm_list.first() {
            let addr = first["address"].as_str().unwrap_or("").to_string();
            if !addr.is_empty() {
                return Ok(addr);
            }
        }
    }
    // Fallback: try wallet balance --chain <id> --output json
    let chain_str = chain_id.to_string();
    let output2 = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str, "--output", "json"])
        .output()?;
    let json2: Value = serde_json::from_str(&String::from_utf8_lossy(&output2.stdout))?;
    let addr = json2["data"]["address"].as_str().unwrap_or("").to_string();
    if !addr.is_empty() {
        return Ok(addr);
    }
    anyhow::bail!("Cannot resolve wallet address for chain {}", chain_id)
}

/// Call onchainos wallet contract-call for EVM chains.
/// dry_run=true returns a mock response without broadcasting.
/// NOTE: onchainos does not support --dry-run; we handle it here.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    force: bool,
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
    ];
    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::json!({"ok": false, "error": stdout.to_string()})))
}

/// Extract txHash from onchainos response.
/// Checks data.txHash → txHash (root).
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// ERC-20 approve via wallet contract-call.
/// approve(address,uint256) selector = 0x095ea7b3
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x"));
    let amount_hex = format!("{:0>64x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, true, dry_run).await
}
