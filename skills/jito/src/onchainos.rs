use anyhow::Result;
use serde_json::Value;
use std::process::Command;

/// Resolve the logged-in Solana wallet address.
/// ⚠️  Solana does NOT support `--output json` flag on `wallet balance`.
///     The address is at data.details[0].tokenAssets[0].address
pub fn resolve_wallet_solana() -> Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet balance: {}\nOutput: {}", e, stdout))?;

    // Try details[0].tokenAssets[0].address first
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        return Ok(addr.to_string());
    }
    // Fallback to data.address
    json["data"]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve Solana wallet address. Make sure onchainos is logged in."))
}

/// Submit a Solana transaction via onchainos.
/// serialized_tx: base64-encoded transaction (from our builder)
/// program_id: the target program (to field)
/// dry_run: if true, return simulated response without broadcasting
///
/// ⚠️  onchainos --unsigned-tx expects BASE58; we convert from base64 internally
/// ⚠️  MUST add --force otherwise returns txHash:"pending" and never broadcasts
pub async fn wallet_contract_call_solana(
    program_id: &str,
    serialized_tx: &str, // base64-encoded
    dry_run: bool,
) -> Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "serialized_tx": serialized_tx
        }));
    }

    // Convert base64 → base58 (onchainos requires base58)
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let tx_bytes = BASE64
        .decode(serialized_tx)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 tx: {}", e))?;
    let tx_base58 = bs58::encode(&tx_bytes).into_string();

    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            program_id,
            "--unsigned-tx",
            &tx_base58,
            "--force",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}\nOutput: {}", e, stdout))
}

/// Extract txHash from onchainos response.
/// Checks: data.swapTxHash → data.txHash → txHash (root)
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
