use anyhow::Context;
use serde_json::Value;
use std::process::Command;

use crate::config::WMATIC;
use crate::rpc;

/// Normalize a token symbol/address to (address, decimals).
/// - If it looks like an address (0x + 42 chars), return as-is with RPC decimals lookup.
/// - If "MATIC", "POL", "WPOL" → return WMATIC address.
/// - Otherwise, call `onchainos token search --chain polygon --symbol <sym>`.
pub async fn resolve_token(
    symbol_or_addr: &str,
    _chain_id: u64,
) -> anyhow::Result<(String, u8)> {
    let input = symbol_or_addr.trim();

    // Native MATIC / POL / WPOL → WMATIC
    let upper = input.to_uppercase();
    if upper == "MATIC" || upper == "POL" || upper == "WPOL" || upper == "WMATIC" {
        let decimals = rpc::get_erc20_decimals(WMATIC, crate::config::RPC_URL)
            .await
            .unwrap_or(18);
        return Ok((WMATIC.to_lowercase(), decimals));
    }

    // Already an address
    if input.starts_with("0x") && input.len() == 42 {
        let decimals = rpc::get_erc20_decimals(input, crate::config::RPC_URL)
            .await
            .unwrap_or(18);
        return Ok((input.to_lowercase(), decimals));
    }

    // Resolve via onchainos token search (uses --query flag, returns {"ok":true,"data":[...]})
    let output = Command::new("onchainos")
        .args([
            "token",
            "search",
            "--chain",
            "polygon",
            "--query",
            input,
        ])
        .output()
        .context("Failed to run onchainos token search")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: Value = serde_json::from_str(&stdout)
        .map_err(|_| anyhow::anyhow!("Could not resolve token '{}'. Check the symbol and try again, or pass the token address directly.", input))?;

    // Response: {"ok": true, "data": [{...token...}, ...]}
    let token = val["data"]
        .as_array()
        .and_then(|arr| arr.first())
        .cloned()
        .or_else(|| {
            // Fallback: flat array at root
            val.as_array().and_then(|arr| arr.first()).cloned()
        })
        .ok_or_else(|| anyhow::anyhow!("Token '{}' not found. Use the token address directly (e.g. 0x2791Bca1...) or check the symbol spelling.", input))?;

    // Field names from onchainos: tokenContractAddress, decimal (not address/decimals)
    let address = token["tokenContractAddress"]
        .as_str()
        .or_else(|| token["address"].as_str())
        .ok_or_else(|| anyhow::anyhow!("Token '{}' resolved but has no address field", input))?
        .to_lowercase();

    let decimals = token["decimal"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| token["decimals"].as_u64())
        .unwrap_or(18) as u8;

    Ok((address, decimals))
}

/// Get the user's wallet address for a given chain.
pub fn wallet_address(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "addresses",
            "--chain",
            &chain_id.to_string(),
        ])
        .output()
        .context("Failed to run onchainos wallet addresses")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let val: Value = serde_json::from_str(&stdout)
        .map_err(|_| anyhow::anyhow!("Failed to parse wallet addresses response"))?;

    // Response: {"ok":true,"data":{"evm":[{"address":"0x...","chainIndex":"137",...}],...}}
    let addr = val["data"]["evm"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v["address"].as_str())
        .map(|s| s.to_string())
        // Fallback: flat array or simple {"address":"..."}
        .or_else(|| {
            val.as_array()
                .and_then(|arr| arr.first())
                .and_then(|v| v["address"].as_str().or_else(|| v.as_str()))
                .map(|s| s.to_string())
        })
        .or_else(|| val["address"].as_str().map(|s| s.to_string()));

    addr.ok_or_else(|| anyhow::anyhow!("No wallet address found for chain {}. Run: onchainos wallet addresses --chain {}", chain_id, chain_id))
}

/// Execute a contract call via onchainos.
/// dry_run = true → return a preview JSON without broadcasting.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "dryRun": true,
            "to": to,
            "inputData": &data[..data.len().min(66)],
            "chain": chain_id,
            "status": "preview — pass --confirm to broadcast"
        }));
    }

    let mut args = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        data.to_string(),
    ];

    if let Some(f) = from {
        args.push("--from".to_string());
        args.push(f.to_string());
    }
    args.push("--biz-type".to_string());
    args.push("dapp".to_string());
    args.push("--strategy".to_string());
    args.push("quickswap-plugin".to_string());

    let output = Command::new("onchainos")
        .args(&args)
        .output()
        .context("Failed to run onchainos wallet contract-call")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "onchainos contract-call failed: {}",
            stderr.trim()
        ));
    }

    serde_json::from_str(&stdout)
        .map_err(|_| anyhow::anyhow!("Failed to parse contract-call response: {}", stdout.trim()))
}

/// Execute a payable contract call with a native value amount (in wei).
pub fn wallet_contract_call_with_value(
    chain_id: u64,
    to: &str,
    data: &str,
    from: Option<&str>,
    value: u128,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "dryRun": true,
            "to": to,
            "inputData": &data[..data.len().min(66)],
            "value": value.to_string(),
            "chain": chain_id,
            "status": "preview — pass --confirm to broadcast"
        }));
    }

    let mut args = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        data.to_string(),
        "--amt".to_string(),
        value.to_string(),
    ];

    if let Some(f) = from {
        args.push("--from".to_string());
        args.push(f.to_string());
    }
    args.push("--biz-type".to_string());
    args.push("dapp".to_string());
    args.push("--strategy".to_string());
    args.push("quickswap-plugin".to_string());

    let output = Command::new("onchainos")
        .args(&args)
        .output()
        .context("Failed to run onchainos wallet contract-call")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "onchainos contract-call (payable) failed: {}",
            stderr.trim()
        ));
    }

    serde_json::from_str(&stdout)
        .map_err(|_| anyhow::anyhow!("Failed to parse contract-call response: {}", stdout.trim()))
}
