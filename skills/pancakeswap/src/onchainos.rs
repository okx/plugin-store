/// Wrapper for `onchainos wallet contract-call` CLI.

pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    _dry_run: bool,
) -> anyhow::Result<serde_json::Value> {
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
        "--force",
    ];

    let amt_str: String;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }

    let from_str: String;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }

    let out = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        // Return stderr in a structured way for debugging
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("onchainos returned empty output. stderr: {}", stderr);
    }

    let v: serde_json::Value = serde_json::from_str(&stdout)?;
    // Propagate onchainos-level errors so callers can see and handle them
    if v.get("ok").and_then(|b| b.as_bool()) == Some(false) {
        let msg = v.get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown onchainos error");
        eprintln!("  [onchainos error] {}", msg);
        // Return the value anyway so the caller can decide how to proceed
    }
    Ok(v)
}

pub fn extract_tx_hash(r: &serde_json::Value) -> &str {
    r["data"]["txHash"]
        .as_str()
        .or_else(|| r["txHash"].as_str())
        .unwrap_or("pending")
}

/// Fetch the wallet's EVM address for a given chain via `onchainos wallet addresses`.
/// Returns the first EVM address found (all chains share the same EVM address).
pub async fn get_wallet_address() -> anyhow::Result<String> {
    let out = tokio::process::Command::new("onchainos")
        .args(&["wallet", "addresses"])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet addresses: {}", e))?;
    v["data"]["evm"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not find EVM address in wallet addresses response"))
}
