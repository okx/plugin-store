use serde_json::Value;
use std::process::Command;

/// Resolve the active EVM wallet address for the given chain.
/// If dry_run is true, returns the zero address without calling onchainos.
pub fn resolve_wallet(chain_id: u64, dry_run: bool) -> anyhow::Result<String> {
    if dry_run {
        return Ok("0x0000000000000000000000000000000000000000".to_string());
    }
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

/// Call `onchainos wallet contract-call` and return parsed JSON output.
/// In dry_run mode: prints the simulated command and returns a fake success response.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    _from: Option<&str>,
    amt_wei: Option<u128>,
    dry_run: bool,
) -> anyhow::Result<Value> {
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
    if let Some(v) = amt_wei {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }

    if dry_run {
        eprintln!("[euler-v2] [dry-run] Would run: onchainos {}", args.join(" "));
        return Ok(serde_json::json!({
            "ok": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }
        }));
    }

    // --force required for all on-chain write operations
    args.push("--force".to_string());

    let output = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {}. Raw: {}", e, stdout))?;
    Ok(result)
}

/// Extract txHash from wallet contract-call response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}

/// ERC-20 approve(address spender, uint256 amount)
/// selector: 0x095ea7b3
#[allow(dead_code)]
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_clean = spender.trim_start_matches("0x").to_lowercase();
    let calldata = format!(
        "0x095ea7b3{:0>64}{:064x}",
        spender_clean, amount
    );
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}
