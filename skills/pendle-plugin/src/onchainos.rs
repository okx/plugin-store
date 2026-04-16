use serde_json::Value;

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
