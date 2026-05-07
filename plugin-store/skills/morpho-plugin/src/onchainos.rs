use serde_json::Value;

/// Call `onchainos wallet contract-call` and return parsed JSON output.
/// Set `force=true` to append `--force` and broadcast immediately (use only for token approvals).
/// For main protocol operations (supply, borrow, repay, withdraw, claim), use `force=false` —
/// onchainos will present the transaction for user confirmation before broadcasting.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
    dry_run: bool,
    force: bool,
) -> anyhow::Result<Value> {
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
    // In dry-run mode, just print the command that would be executed and return a simulated response.
    if dry_run {
        eprintln!("[morpho] [dry-run] Would run: onchainos {}", args.join(" "));
        return Ok(serde_json::json!({
            "ok": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }
        }));
    }

    if force {
        args.push("--force");
    }

    let output = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Poll until a transaction is confirmed on-chain.
/// Called after approve --force so the main op simulation sees the updated allowance.
/// Uses 20 attempts × 2s = 40s for all chains (Base ~2s blocks still needs headroom for RPC lag).
pub async fn wait_for_tx(tx_hash: &str, rpc_url: &str, _chain_id: u64) -> anyhow::Result<()> {
    if tx_hash == "0x0000000000000000000000000000000000000000000000000000000000000000" {
        return Ok(()); // dry-run stub hash — nothing to wait for
    }
    let max_attempts: u32 = 20; // 40s — same for all chains
    let client = reqwest::Client::new();
    for _ in 0..max_attempts {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let body = serde_json::json!({
            "jsonrpc": "2.0", "method": "eth_getTransactionReceipt",
            "params": [tx_hash], "id": 1
        });
        if let Ok(resp) = client.post(rpc_url).json(&body).send().await {
            if let Ok(v) = resp.json::<serde_json::Value>().await {
                if !v["result"].is_null() {
                    return Ok(());
                }
            }
        }
    }
    anyhow::bail!(
        "Approval tx {} not confirmed within {}s — network may be congested. \
         Check the tx on-chain and retry the command once it confirms.",
        tx_hash,
        max_attempts * 2
    )
}

/// Extract txHash from wallet contract-call response, returning an error if the call failed.
/// Response format: {"ok":true,"data":{"txHash":"0x..."}}
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

/// Encode and submit an ERC-20 approve call.
/// Selector: 0x095ea7b3
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
    // Approvals always use --force: they are prerequisite steps, not the main user action
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run, true).await
}

/// Query wallet balance for the given chain. Returns raw JSON from onchainos.
pub async fn wallet_balance(chain_id: u64) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Resolve the caller's wallet address: use `from` if provided, otherwise
/// query the active onchainos wallet via `wallet addresses --chain <id>`.
pub async fn resolve_wallet(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    let chain_str = chain_id.to_string();
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_str])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}\nraw: {}", e, stdout))?;
    let addr = v["data"]["evm"][0]["address"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not determine active EVM wallet address. Ensure onchainos is logged in."))?
        .to_string();
    Ok(addr)
}
