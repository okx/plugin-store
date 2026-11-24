use std::process::Command;
use serde_json::Value;

/// Resolve the active EVM wallet address for Base (chain 8453).
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|_| anyhow::anyhow!("Could not parse onchainos wallet addresses output"))?;
    let chain_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // fallback: first EVM address
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not determine active EVM wallet address for chain {}", chain_id)
}

/// Execute an on-chain write via `onchainos wallet contract-call`.
/// Requires --force to broadcast. Returns the raw onchainos JSON response.
/// In dry_run mode, returns a stub without calling onchainos.
/// Pass `from` whenever the caller has already resolved the wallet address so
/// onchainos uses the correct signer on multi-wallet setups.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    force: bool,
    dry_run: bool,
    from: Option<&str>,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": input_data,
            "to": to
        }));
    }
    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet", "contract-call",
        "--chain", &chain_str,
        "--to", to,
        "--input-data", input_data,
        "--biz-type", "dapp",
        "--strategy", "aerodrome-slipstream-plugin",
    ];
    if force {
        args.push("--force");
    }
    if let Some(addr) = from {
        args.push("--from");
        args.push(addr);
    }
    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&stdout)
        .map_err(|_| anyhow::anyhow!("onchainos contract-call: unexpected output: {}", stdout))?;
    // Propagate onchainos-level errors (ok: false responses)
    if result["ok"].as_bool() == Some(false) {
        let err_msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("transaction failed");
        anyhow::bail!("onchainos error: {}", err_msg);
    }
    Ok(result)
}

/// Extract txHash from an onchainos wallet_contract_call response.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
