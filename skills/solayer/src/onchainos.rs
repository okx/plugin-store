use std::process::Command;
use serde_json::Value;

/// Resolve the current Solana wallet address from onchainos.
/// ⚠️  Solana does NOT support --output json flag.
/// ⚠️  Address path: data.details[0].tokenAssets[0].address
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {}\nOutput: {}", e, stdout))?;

    // Primary path: data.details[0].tokenAssets[0].address
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // Fallback: data.address
    if let Some(addr) = json["data"]["address"].as_str() {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("Could not resolve Solana wallet address from onchainos output")
}

/// Submit a Solana serialized transaction via onchainos.
/// serialized_tx_base64: base64-encoded transaction (from Solayer API)
/// to: program address (base58)
/// ⚠️  onchainos --unsigned-tx expects base58; we convert from base64 here
/// ⚠️  --force is required for Solana contract-call to broadcast
pub async fn wallet_contract_call_solana(
    to: &str,
    serialized_tx_base64: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "serialized_tx": serialized_tx_base64
        }));
    }

    // Convert base64 → base58 (onchainos requires base58)
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let tx_bytes = BASE64.decode(serialized_tx_base64)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 tx: {}", e))?;
    let tx_base58 = bs58::encode(&tx_bytes).into_string();

    let output = Command::new("onchainos")
        .args([
            "wallet", "contract-call",
            "--chain", "501",
            "--to", to,
            "--unsigned-tx", &tx_base58,
            "--force",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}\nOutput: {}", e, stdout))
}

/// Extract txHash from onchainos response.
/// Priority: data.swapTxHash → data.txHash → txHash (root)
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
