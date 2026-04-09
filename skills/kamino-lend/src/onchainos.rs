use std::process::Command;
use serde_json::Value;

/// Resolve the current Solana wallet address from onchainos.
/// NOTE: Solana does NOT support --output json; wallet balance returns JSON directly.
/// Address path: data.details[0].tokenAssets[0].address
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"]) // no --output json for Solana
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        return Ok(addr.to_string());
    }
    // fallback
    if let Some(addr) = json["data"]["address"].as_str() {
        return Ok(addr.to_string());
    }
    anyhow::bail!("Could not resolve Solana wallet address from onchainos")
}

/// Convert base64-encoded serialized Solana transaction to base58.
/// Kamino API returns base64; onchainos --unsigned-tx expects base58.
pub fn base64_to_base58(b64: &str) -> anyhow::Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let bytes = STANDARD.decode(b64.trim())?;
    Ok(bs58::encode(bytes).into_string())
}

/// Submit a Solana transaction via onchainos wallet contract-call.
/// serialized_tx: base64-encoded transaction (from Kamino API `transaction` field).
/// to: Kamino Lend Program ID (KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD).
/// dry_run: if true, returns simulated response without calling onchainos.
///
/// IMPORTANT: onchainos --unsigned-tx expects base58 encoding; this function
/// performs the base64→base58 conversion internally.
/// IMPORTANT: Solana blockhash expires ~60s; call this immediately after receiving
/// the serialized tx from the API.
pub async fn wallet_contract_call_solana(
    to: &str,
    serialized_tx: &str, // base64-encoded (from Kamino API)
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "serialized_tx": serialized_tx
        }));
    }

    // Convert base64 → base58 (onchainos requires base58)
    let tx_base58 = base64_to_base58(serialized_tx)
        .map_err(|e| anyhow::anyhow!("base64→base58 conversion failed: {}", e))?;

    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            to,
            "--unsigned-tx",
            &tx_base58,
            "--force",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        anyhow::bail!(
            "onchainos contract-call failed (exit code {:?}).\nstdout: {}\nstderr: {}",
            output.status.code(),
            stdout,
            stderr
        );
    }

    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!(
            "Failed to parse onchainos response: {}\nstdout: {}\nstderr: {}",
            e, stdout, stderr
        ))?;

    // Check for API-level errors in the JSON response
    if result.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        let msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        anyhow::bail!("onchainos contract-call returned error: {}", msg);
    }

    Ok(result)
}

/// Extract txHash from onchainos response.
/// Checks data.txHash and data.swapTxHash (for DEX operations).
/// Returns an error if no txHash is found instead of silently returning "pending".
pub fn extract_tx_hash(result: &Value) -> anyhow::Result<String> {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!(
            "onchainos did not return a transaction hash. Response: {}",
            serde_json::to_string(result).unwrap_or_default()
        ))
}
