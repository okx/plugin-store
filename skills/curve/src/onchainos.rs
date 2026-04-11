// onchainos.rs — onchainos CLI wrapper
use std::process::Command;
use serde_json::Value;

/// Resolve active wallet EVM address via `onchainos wallet addresses`.
pub fn resolve_wallet(_chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    Ok(json["data"]["evmAddress"].as_str().unwrap_or("").to_string())
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

/// Poll onchainos wallet history until txStatus is SUCCESS or FAILED (or timeout).
/// Uses spawn_blocking so Command::output() doesn't block the Tokio runtime thread.
pub async fn wait_for_tx(chain_id: u64, tx_hash: String, wallet_addr: String) -> anyhow::Result<bool> {
    tokio::task::spawn_blocking(move || wait_for_tx_sync(chain_id, &tx_hash, &wallet_addr))
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}

fn wait_for_tx_sync(chain_id: u64, tx_hash: &str, wallet_addr: &str) -> anyhow::Result<bool> {
    let chain_str = chain_id.to_string();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        if std::time::Instant::now() > deadline {
            anyhow::bail!("Timeout (90s) waiting for tx {} to confirm", tx_hash);
        }
        let output = Command::new("onchainos")
            .args([
                "wallet", "history",
                "--tx-hash", tx_hash,
                "--address", wallet_addr,
                "--chain", &chain_str,
            ])
            .output();
        if let Ok(out) = output {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&out.stdout)) {
                if let Some(entry) = v["data"].as_array().and_then(|a| a.first()) {
                    match entry["txStatus"].as_str() {
                        Some("SUCCESS") => return Ok(true),
                        Some("FAILED") => {
                            let reason = entry["failReason"].as_str().unwrap_or("");
                            anyhow::bail!("tx {} failed on-chain: {}", tx_hash, reason);
                        }
                        _ => {} // PENDING — keep polling
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
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
    // approve does not need --force (only swap/exchange does)
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, false, dry_run).await
}
