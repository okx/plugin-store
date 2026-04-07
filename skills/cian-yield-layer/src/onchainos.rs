use std::process::Command;
use serde_json::Value;

/// Resolve the currently logged-in EVM wallet address on chain 1.
/// Uses 3-level fallback: data.details[0].tokenAssets[0].address
///   → data.address → wallet addresses (data.evm[])
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
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

    let addr = json["data"]["address"].as_str().unwrap_or("").to_string();
    if !addr.is_empty() {
        return Ok(addr);
    }

    // fallback 3: wallet addresses (zero-balance wallet)
    let addr_output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let addr_json: Value = serde_json::from_str(&String::from_utf8_lossy(&addr_output.stdout))
        .unwrap_or_default();
    let chain_id_str = chain_id.to_string();
    if let Some(evm_addrs) = addr_json["data"]["evm"].as_array() {
        for entry in evm_addrs {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(a) = entry["address"].as_str() {
                    if !a.is_empty() {
                        return Ok(a.to_string());
                    }
                }
            }
        }
        for entry in evm_addrs {
            if let Some(a) = entry["address"].as_str() {
                if !a.is_empty() {
                    return Ok(a.to_string());
                }
            }
        }
    }
    Ok(String::new())
}

/// Submit an EVM transaction via onchainos wallet contract-call.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    wallet_contract_call_inner(chain_id, to, input_data, from, amt, None, false, dry_run).await
}

/// Submit an EVM transaction with optional gas limit and force flag.
pub async fn wallet_contract_call_force(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    gas_limit: Option<u64>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    wallet_contract_call_inner(chain_id, to, input_data, from, amt, gas_limit, true, dry_run).await
}

async fn wallet_contract_call_inner(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    gas_limit: Option<u64>,
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
        "wallet", "contract-call",
        "--chain", &chain_str,
        "--to", to,
        "--input-data", input_data,
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
    let gas_str;
    if let Some(g) = gas_limit {
        gas_str = g.to_string();
        args.extend_from_slice(&["--gas-limit", &gas_str]);
    }
    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from onchainos response (EVM).
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// ERC-20 approve — manual calldata encoding (no onchainos dex approve command).
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_padded = format!("{:0>64}", spender.strip_prefix("0x").unwrap_or(spender).to_lowercase());
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}
