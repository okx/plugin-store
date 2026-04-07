// onchainos CLI wrapper — Stader plugin
// All on-chain writes go through `onchainos wallet contract-call --input-data`
// Never use --dry-run flag with onchainos — handle dry_run in Rust before calling CLI

use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in EVM wallet address for a given chain.
/// Uses `onchainos wallet balance --chain <id>` (no --output json — not supported on all chains).
/// Address is read from data.details[0].tokenAssets[0].address.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;

    // Try data.details[0].tokenAssets[0].address first
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // Fallback: data.address
    let addr = json["data"]["address"].as_str().unwrap_or("").to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address. Is onchainos logged in on chain {}?", chain_id);
    }
    Ok(addr)
}

/// Submit an EVM contract call via `onchainos wallet contract-call`.
/// dry_run=true returns a simulated response without calling onchainos.
/// NOTE: onchainos contract-call does NOT support --dry-run; handle it here.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    amt: Option<u64>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
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

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| {
        anyhow::anyhow!("Failed to parse onchainos output: {}\nOutput: {}", e, stdout)
    })
}

/// ERC-20 approve calldata: approve(address spender, uint256 amount)
/// selector = 0x095ea7b3
pub fn erc20_approve_calldata(spender: &str, amount: u128) -> String {
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}

/// Extract txHash from onchainos response.
/// Checks data.txHash first, then root txHash.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
