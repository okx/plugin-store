use anyhow::Result;
use serde_json::Value;
use std::process::Command;

/// Resolve EVM wallet address for the given chain.
/// Uses `onchainos wallet balance --chain <id>` (no --output json on chain 1).
/// Address path: data.details[0].tokenAssets[0].address
pub fn resolve_wallet(chain_id: u64) -> Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet balance output: {}\nOutput: {}", e, stdout))?;

    // primary path
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // fallback
    let addr = json["data"]["address"].as_str().unwrap_or("").to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address. Ensure onchainos is logged in.");
    }
    Ok(addr)
}

/// Submit an EVM contract call via `onchainos wallet contract-call`.
/// dry_run: returns a simulated response without calling onchainos.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>, // wei value (for native ETH sends)
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

    let from_owned;
    if let Some(f) = from {
        from_owned = f.to_string();
        args.extend_from_slice(&["--from", &from_owned]);
    }

    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse contract-call output: {}\nOutput: {}", e, stdout))?;
    Ok(json)
}

/// Extract txHash from onchainos response.
/// Checks data.txHash -> data.swapTxHash -> root txHash in order.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["data"]["swapTxHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// ERC-20 approve via wallet contract-call.
/// Encodes approve(address spender, uint256 amount) calldata manually.
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> Result<Value> {
    // approve(address,uint256) selector = 0x095ea7b3
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run, false).await
}

/// Read ERC-20 allowance via eth_call.
/// Returns the allowance as u128 (0 on error).
pub async fn erc20_allowance(
    _chain_id: u64,
    token_addr: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> u128 {
    // allowance(address owner, address spender) selector = 0xdd62ed3e
    let owner_clean = owner.trim_start_matches("0x");
    let spender_clean = spender.trim_start_matches("0x");
    let calldata = format!(
        "0xdd62ed3e{:0>64}{:0>64}",
        owner_clean, spender_clean
    );

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [{"to": token_addr, "data": calldata}, "latest"]
    });

    let client = crate::api::build_client();
    let resp = match client.post(rpc_url).json(&body).send().await {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let json: Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return 0,
    };

    let hex = json["result"].as_str().unwrap_or("0x0");
    let hex_clean = hex.trim_start_matches("0x");
    u128::from_str_radix(hex_clean, 16).unwrap_or(0)
}
