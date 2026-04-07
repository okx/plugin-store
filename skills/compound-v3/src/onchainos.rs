// src/onchainos.rs
use std::process::Command;
use serde_json::Value;

/// Query the currently logged-in wallet address for the given chain.
///
/// Calls `onchainos wallet addresses` which returns:
/// { "data": { "evm": [ { "chainIndex": "8453", "address": "0x..." }, ... ] } }
/// Returns the EVM address whose chainIndex matches chain_id.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_index = chain_id.to_string();
    let evm_entries = json["data"]["evm"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("onchainos wallet addresses: missing data.evm array"))?;
    for entry in evm_entries {
        if entry["chainIndex"].as_str() == Some(&chain_index) {
            if let Some(addr) = entry["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!(
        "No EVM address found for chainIndex={} in onchainos wallet addresses output",
        chain_id
    )
}

/// Submit a contract call via onchainos wallet contract-call.
/// ⚠️  dry_run=true returns a simulated response immediately — contract-call does NOT support --dry-run.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>, // wei value attached (e.g. WETH deposit)
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
    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }
    let from_str;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        let err = if !stderr.is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
        anyhow::bail!("onchainos contract-call failed (exit {}): {}", output.status, err);
    }
    let result: Value = serde_json::from_str(&stdout)?;
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str().unwrap_or("unknown onchainos error");
        anyhow::bail!("onchainos execution failed: {}", err_msg);
    }
    Ok(result)
}

/// Extract txHash from wallet contract-call response: {"ok":true,"data":{"txHash":"0x..."}}
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    let hash = result["data"]["swapTxHash"].as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str());
    match hash {
        Some(h) if !h.is_empty() && h != "pending" => Ok(h.to_string()),
        _ => anyhow::bail!("txHash not found in onchainos output; raw: {}", result),
    }
}

/// ERC-20 approve via wallet contract-call (approve(address,uint256) selector = 0x095ea7b3)
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128, // u128::MAX for unlimited
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_padded = format!("{:0>64}", &spender[2..]);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}

