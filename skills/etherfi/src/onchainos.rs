use std::process::Command;
use serde_json::Value;

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
