use std::process::Command;
use anyhow::Context;

/// Resolve the active wallet address for the given chain via onchainos.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let out = Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", &chain_id.to_string()])
        .output()
        .context("Failed to run `onchainos wallet addresses`")?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .context("Failed to parse onchainos wallet addresses output")?;

    // Try data.evm[].address where chainIndex matches
    if let Some(evm_list) = v.pointer("/data/evm").and_then(|e| e.as_array()) {
        for entry in evm_list {
            if let Some(addr) = entry.get("address").and_then(|a| a.as_str()) {
                if !addr.is_empty() {
                    return Ok(addr.to_string());
                }
            }
        }
    }
    // Fallback: flat address field
    if let Some(addr) = v.get("address").and_then(|a| a.as_str()) {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("Could not determine active EVM wallet address for chain {}", chain_id)
}

/// Call a contract via onchainos wallet contract-call.
/// Returns the parsed JSON response.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    amt_eth: &str,
    dry_run: bool,
    from: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "data": {"txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"}
        }));
    }

    let mut args = vec![
        "wallet".to_string(), "contract-call".to_string(),
        "--chain".to_string(), chain_id.to_string(),
        "--to".to_string(), to.to_string(),
        "--input-data".to_string(), input_data.to_string(),
        "--amt".to_string(), amt_eth.to_string(),
        "--biz-type".to_string(), "dapp".to_string(),
        "--strategy".to_string(), "fluid-plugin".to_string(),
    ];
    if let Some(addr) = from {
        args.push("--from".to_string());
        args.push(addr.to_string());
    }

    let out = Command::new("onchainos")
        .args(&args)
        .output()
        .context("Failed to run `onchainos wallet contract-call`")?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or(serde_json::json!({"ok": false, "error": stdout.trim()}));

    if v.get("ok").and_then(|o| o.as_bool()) == Some(false) {
        let msg = v.get("error").and_then(|e| e.as_str()).unwrap_or("unknown error");
        anyhow::bail!("onchainos error: {}", msg);
    }
    Ok(v)
}

/// Extract tx hash from onchainos response.
pub fn extract_tx_hash(v: &serde_json::Value) -> String {
    v.pointer("/data/txHash")
        .or_else(|| v.get("txHash"))
        .and_then(|h| h.as_str())
        .unwrap_or("0x0000000000000000000000000000000000000000000000000000000000000000")
        .to_string()
}
