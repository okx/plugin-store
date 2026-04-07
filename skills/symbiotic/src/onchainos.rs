use std::process::Command;
use serde_json::Value;

/// Resolve EVM wallet address via onchainos wallet addresses (chainIndex for Ethereum = "1")
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    // Use 'wallet addresses' — works on all chains including Ethereum mainnet
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet addresses: {} — stdout: {}", e, &stdout[..stdout.len().min(200)]))?;
    // Find the EVM address matching chainIndex == chain_id
    if let Some(evm_arr) = json["data"]["evm"].as_array() {
        for entry in evm_arr {
            if entry["chainIndex"].as_str() == Some(&chain_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // Fallback: return first EVM address
        if let Some(first) = evm_arr.first() {
            return Ok(first["address"].as_str().unwrap_or("").to_string());
        }
    }
    // Last fallback: try wallet balance --chain 8453 (Base) which supports --output json
    let base_output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "8453", "--output", "json"])
        .output()?;
    let base_stdout = String::from_utf8_lossy(&base_output.stdout);
    let base_json: Value = serde_json::from_str(&base_stdout).unwrap_or(Value::Null);
    Ok(base_json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Call onchainos wallet contract-call for EVM write operations.
/// dry_run=true → returns simulated response, does NOT call onchainos.
/// onchainos wallet contract-call does NOT support --dry-run flag.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
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
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_str.clone(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
    ];
    if let Some(v) = amt {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }
    if let Some(f) = from {
        args.push("--from".to_string());
        args.push(f.to_string());
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {} — stdout: {}", e, &stdout[..stdout.len().min(300)]))?)
}

/// ERC-20 approve calldata builder and submitter.
/// approve(address,uint256) selector: 0x095ea7b3 [verified: cast sig "approve(address,uint256)"]
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}

/// Extract txHash from onchainos response.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
