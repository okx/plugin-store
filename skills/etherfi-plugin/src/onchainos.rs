use std::process::Command;
use serde_json::Value;

/// Poll onchainos wallet history until txStatus is SUCCESS or FAILED (or 90s timeout).
/// Uses spawn_blocking so Command::output() doesn't block the Tokio runtime thread.
pub async fn wait_for_tx(tx_hash: String, wallet_addr: String) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || wait_for_tx_sync(&tx_hash, &wallet_addr))
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}

fn wait_for_tx_sync(tx_hash: &str, wallet_addr: &str) -> anyhow::Result<()> {
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
                "--chain", "1",
            ])
            .output();
        if let Ok(out) = output {
            if let Ok(v) = serde_json::from_str::<Value>(&String::from_utf8_lossy(&out.stdout)) {
                if let Some(entry) = v["data"].as_array().and_then(|a| a.first()) {
                    match entry["txStatus"].as_str() {
                        Some("SUCCESS") => return Ok(()),
                        Some("FAILED") => {
                            let reason = entry["failReason"].as_str().unwrap_or("");
                            anyhow::bail!("approve tx {} failed on-chain: {}", tx_hash, reason);
                        }
                        _ => {} // PENDING — keep polling
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    }
}

/// Resolve the EVM wallet address for Ethereum (chain_id=1) from the onchainos CLI.
/// Parses `onchainos wallet addresses` JSON and returns the first matching EVM address.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    let chain_id_str = chain_id.to_string();
    if let Some(evm_list) = json["data"]["evm"].as_array() {
        for entry in evm_list {
            if entry["chainIndex"].as_str() == Some(&chain_id_str) {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // Fallback: use first EVM address
        if let Some(first) = evm_list.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve wallet address for chain {}", chain_id)
}

/// Execute a contract call via `onchainos wallet contract-call`.
///
/// Parameters:
/// - `chain_id`    — Ethereum chain ID (1 for mainnet)
/// - `to`          — target contract address
/// - `input_data`  — ABI-encoded calldata (0x-prefixed hex)
/// - `value_wei`   — native ETH to send as msg.value (0 for non-payable calls)
/// - `confirm`     — if false, returns a preview JSON without broadcasting;
///                   if true, broadcasts the transaction
/// - `dry_run`     — if true, returns mock response without calling onchainos
///
/// **Confirm gate**: Write operations always preview first. The caller must pass
/// `confirm=true` (via `--confirm` flag) to actually broadcast.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    value_wei: u128,
    confirm: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {"txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"},
            "calldata": input_data,
            "value": value_wei.to_string()
        }));
    }

    if !confirm {
        // Preview mode: show what would be sent but do NOT broadcast
        return Ok(serde_json::json!({
            "ok": true,
            "preview": true,
            "message": "Run with --confirm to broadcast this transaction.",
            "to": to,
            "calldata": input_data,
            "value_wei": value_wei.to_string(),
            "chain_id": chain_id
        }));
    }

    let chain_str = chain_id.to_string();
    let value_str = value_wei.to_string();
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            &chain_str,
            "--to",
            to,
            "--input-data",
            input_data,
            "--amt",
            &value_str,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::json!({"ok": false, "raw": stdout.to_string()}));

    // Propagate ok:false as an error — prevents "pending" txHash on simulation rejection
    if result["ok"].as_bool() == Some(false) {
        let msg = result["message"].as_str()
            .or_else(|| result["data"]["message"].as_str())
            .or_else(|| result["raw"].as_str())
            .unwrap_or("onchainos wallet contract-call failed (ok: false)");
        anyhow::bail!("{}", msg);
    }

    Ok(result)
}

/// Extract txHash from a wallet_contract_call response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
