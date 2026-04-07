/// onchainos CLI wrapper

use anyhow::Result;
use serde_json::Value;
use std::process::Command;

/// Resolve the current wallet address for a given EVM chain.
/// `onchainos wallet balance --chain <ID>` does NOT support --output json for EVM chains;
/// it returns JSON directly without the flag.
pub fn resolve_wallet(chain_id: u64) -> Result<String> {
    let chain_str = chain_id.to_string();
    // Note: No --output json flag — wallet balance returns JSON natively for EVM chains
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet balance JSON: {}", e))?;
    // Address is nested in data.details[0].tokenAssets[0].address
    let addr = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
        .unwrap_or_else(|| json["data"]["address"].as_str().unwrap_or(""))
        .to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address. Is onchainos logged in?");
    }
    Ok(addr)
}

/// Submit an EVM contract call via onchainos wallet contract-call.
/// dry_run=true returns a simulated response without calling onchainos.
/// ⚠️  onchainos wallet contract-call does NOT accept --dry-run flag.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>, // wei value for ETH-valued calls
    dry_run: bool,
    force: bool,
) -> Result<Value> {
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
    let from_str;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }
    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}\nOutput: {}", e, stdout))
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

/// ERC-20 approve via wallet contract-call
/// approve(address,uint256) selector = 0x095ea7b3
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> Result<Value> {
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run, true).await
}
