use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in Solana wallet address (base58).
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .map_err(|e| anyhow::anyhow!("wallet addresses parse error: {}", e))?;
    let addr = json["data"]["solana"][0]["address"].as_str().unwrap_or("").to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve Solana wallet address — ensure onchainos is logged in");
    }
    Ok(addr)
}

/// Submit a Solana serialized transaction via onchainos.
/// serialized_tx: base64-encoded VersionedTransaction from Raydium API.
/// onchainos --unsigned-tx expects base58, so we convert here.
/// NOTE: Solana blockhash expires in ~60s — call immediately after receiving tx.
/// NOTE: --force is required for Solana --unsigned-tx submissions to broadcast.
pub async fn wallet_contract_call_solana(
    to: &str,
    serialized_tx: &str,
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
    // onchainos --unsigned-tx expects base58; Raydium API returns base64
    let tx_bytes = BASE64.decode(serialized_tx)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 tx: {}", e))?;
    let tx_base58 = bs58::encode(&tx_bytes).into_string();

    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            to,
            "--unsigned-tx",
            &tx_base58,
            "--force", // required for Solana --unsigned-tx to broadcast
        ])
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from onchainos response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
