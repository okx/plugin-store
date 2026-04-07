// onchainos.rs — onchainos CLI wrapper
use std::process::Command;
use serde_json::Value;

/// Resolve active wallet address from onchainos wallet balance
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str, "--output", "json"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Call onchainos wallet contract-call.
/// dry_run=true returns a simulated response without calling onchainos.
/// NOTE: onchainos wallet contract-call does NOT support --dry-run.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
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
    let mut args: Vec<String> = vec![
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
    if force {
        args.push("--force".to_string());
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let err = if !stderr.is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
        anyhow::bail!("onchainos failed (exit {}): {}", output.status, err);
    }
    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {}", e))?;
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str().unwrap_or("unknown onchainos error");
        anyhow::bail!("onchainos execution failed: {}", err_msg);
    }
    Ok(result)
}

/// Extract txHash from wallet contract-call response.
/// Returns an error if the hash is missing, empty, or "pending".
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    let hash = result["data"]["swapTxHash"].as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str());
    match hash {
        Some(h) if !h.is_empty() && h != "pending" => Ok(h.to_string()),
        _ => anyhow::bail!("txHash not found in onchainos output; raw: {}", result),
    }
}

/// ERC-20 approve via wallet contract-call
/// Encodes approve(address,uint256) calldata manually (no onchainos dex approve)
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // approve(address,uint256) selector = 0x095ea7b3
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    // approve does not need --force (only swap/exchange does)
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, false, dry_run).await
}
