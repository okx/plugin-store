use serde_json::Value;

/// Get the currently logged-in wallet address for the given chain.
pub async fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = tokio::process::Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_str])
        .output()
        .await?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}", e))?;
    let addr = json["data"]["evm"][0]["address"].as_str().unwrap_or("").to_string();
    Ok(addr)
}

/// Submit a transaction via `onchainos wallet contract-call`.
///
/// dry_run=true returns a simulated response without calling onchainos.
/// NOTE: `onchainos wallet contract-call` does NOT accept a --dry-run flag.
/// The DEX/farming operations require --force to avoid "txHash: pending" non-broadcast.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
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
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_str.clone(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
    ];

    if let Some(f) = from {
        args.push("--from".to_string());
        args.push(f.to_string());
    }

    if let Some(v) = amt {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }

    let output = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::json!({ "ok": false, "error": stdout.to_string() })))
}

/// Extract txHash from a wallet contract-call response, or return an error if the call failed.
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
