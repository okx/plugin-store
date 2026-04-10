// onchainos.rs — onchainos CLI wrapper
use std::process::Command;
use serde_json::Value;

/// Resolve active wallet EVM address via `onchainos wallet addresses --chain <id>`.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}", e))?;
    let addr = json["data"]["evm"][0]["address"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not determine active EVM wallet address. Ensure onchainos is logged in."))?
        .to_string();
    Ok(addr)
}

/// Poll until a transaction is confirmed on-chain (up to ~24 seconds).
/// Called after approve --force so the main op simulation sees the updated allowance.
pub async fn wait_for_tx(tx_hash: &str, rpc_url: &str) -> anyhow::Result<()> {
    if tx_hash == "0x0000000000000000000000000000000000000000000000000000000000000000" {
        return Ok(()); // dry-run stub hash — nothing to wait for
    }
    let client = reqwest::Client::new();
    for _ in 0..12 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let body = serde_json::json!({
            "jsonrpc": "2.0", "method": "eth_getTransactionReceipt",
            "params": [tx_hash], "id": 1
        });
        if let Ok(resp) = client.post(rpc_url).json(&body).send().await {
            if let Ok(v) = resp.json::<Value>().await {
                if !v["result"].is_null() {
                    return Ok(());
                }
            }
        }
    }
    Ok(()) // proceed anyway after timeout (~24s)
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
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from wallet contract-call response, returning an error on failure.
pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        return Err(anyhow::anyhow!("contract-call failed: {}", err_msg));
    }
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no txHash in contract-call response"))
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
    // Approvals use --force: they must broadcast immediately as prerequisite steps
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, true, dry_run).await
}
