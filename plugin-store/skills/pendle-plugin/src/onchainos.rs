use serde_json::Value;

/// Public RPC endpoints for supported chains — used by wait_for_tx.
pub fn default_rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1     => "https://ethereum.publicnode.com",
        42161 => "https://arbitrum-one-rpc.publicnode.com",
        56    => "https://bsc.publicnode.com",
        8453  => "https://base-rpc.publicnode.com",
        _     => "https://ethereum.publicnode.com",
    }
}

/// Poll eth_getTransactionReceipt until the tx confirms or timeout (20 × 2s = 40s).
/// Called after every ERC-20 approve so the on-chain allowance is visible before
/// the main Pendle router tx fires.  Silently returns on timeout — the router tx
/// will either succeed (allowance landed) or fail with a clear on-chain revert.
pub async fn wait_for_tx(tx_hash: &str, rpc_url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..20u32 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });
        if let Ok(resp) = client.post(rpc_url).json(&body).send().await {
            if let Ok(json) = resp.json::<Value>().await {
                if json.get("result").map(|r| !r.is_null()).unwrap_or(false) {
                    return;
                }
            }
        }
    }
}

/// Validate that an address looks like a well-formed EVM address (0x + 40 hex chars).
pub fn validate_evm_address(addr: &str) -> anyhow::Result<()> {
    if !addr.starts_with("0x") || addr.len() != 42 {
        anyhow::bail!(
            "Invalid EVM address '{}': expected 0x-prefixed 42-character hex string",
            addr
        );
    }
    Ok(())
}

/// Validate that a wei amount string is a positive integer.
pub fn validate_amount(amount: &str, field: &str) -> anyhow::Result<()> {
    let n: u128 = amount
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount for {}: '{}' is not a valid integer", field, amount))?;
    if n == 0 {
        anyhow::bail!("{} must be greater than zero", field);
    }
    Ok(())
}

/// Resolve the current logged-in wallet address for the given chain.
/// Uses `wallet addresses --chain <id>` (EVM path).
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = std::process::Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}", e))?;
    let addr = json["data"]["evm"][0]["address"]
        .as_str()
        .unwrap_or("")
        .to_string();
    Ok(addr)
}

/// Submit a transaction via `onchainos wallet contract-call`.
/// dry_run=true returns a simulated response without calling onchainos.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>,
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
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_str,
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
        // --force bypasses onchainos's interactive confirmation prompt.
        // This is intentional: the plugin implements its own preview/confirm gate.
        // dry_run=true returns early above, so --force is only present on the live
        // execution path (confirm=true), never on preview or dry-run paths.
        "--force".to_string(),
    ];

    if let Some(v) = amt {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }

    if let Some(f) = from {
        args.push("--from".to_string());
        args.push(f.to_string());
    }

    let output = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        anyhow::bail!(
            "onchainos contract-call failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            if stderr.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            }
        );
    }

    let raw = stdout.trim();
    if raw.is_empty() {
        anyhow::bail!(
            "onchainos contract-call returned empty output; stderr: {}",
            stderr.trim()
        );
    }

    serde_json::from_str(raw).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse onchainos output: {}; stdout: {}; stderr: {}",
            e,
            raw,
            stderr.trim()
        )
    })
}

/// Extract txHash from onchainos wallet contract-call response.
/// Returns an error if txHash is absent — a missing hash means the transaction was not broadcast.
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!(
            "Transaction was not broadcast — no txHash in onchainos response: {}",
            result
        ))
}

/// Query ERC-20 balanceOf(wallet) for a given token via a direct JSON-RPC eth_call.
/// Used as a pre-flight balance check before calling the Pendle SDK — surfaces
/// insufficient-balance errors locally rather than spending a round-trip to the SDK.
/// Returns 0 on any RPC error (non-fatal: on-chain will revert if truly underfunded).
pub async fn erc20_balance_of(chain_id: u64, token_addr: &str, wallet: &str) -> anyhow::Result<u128> {
    let rpc_url = default_rpc_url(chain_id);
    let wallet_clean = wallet.strip_prefix("0x").unwrap_or(wallet);
    // balanceOf(address) selector = 0x70a08231; wallet padded to 32 bytes
    let data = format!("0x70a08231{:0>64}", wallet_clean);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": token_addr, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("eth_call for balanceOf failed: {}", e))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse balanceOf response: {}", e))?;
    let hex = resp["result"].as_str().unwrap_or("0x0");
    let clean = hex.trim_start_matches("0x");
    if clean.is_empty() {
        return Ok(0);
    }
    // ABI result is a 32-byte (64 hex char) padded uint256; u128 fits in the last 32 hex chars
    let truncated = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(truncated, 16).unwrap_or(0))
}

/// Build ERC-20 approve calldata and submit via wallet contract-call.
/// approve(address,uint256) selector = 0x095ea7b3
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_clean = spender.strip_prefix("0x").unwrap_or(spender);
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}
