use std::process::Command;
use serde_json::Value;

/// Native SOL mint sentinel used by onchainos for native SOL.
const NATIVE_SOL_MINT: &str = "11111111111111111111111111111111";

/// `--biz-type` / `--strategy`: attribution to the onchainos backend.
/// Source-of-truth for the plugin name is Cargo.toml's `[package]` `name`.
const BIZ_TYPE: &str = "dapp";
const STRATEGY: &str = env!("CARGO_PKG_NAME");

/// Return native SOL balance in UI units (e.g. 1.5 = 1.5 SOL). Returns 0.0 on failure.
pub fn get_sol_balance() -> f64 {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()
        .ok();
    let stdout = output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let json: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    if let Some(assets) = json["data"]["details"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|d| d["tokenAssets"].as_array())
    {
        for asset in assets {
            let addr = asset["tokenAddress"].as_str().unwrap_or("");
            if addr.is_empty() || addr == NATIVE_SOL_MINT {
                return asset["balance"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
            }
        }
    }
    0.0
}

/// Return all token balances (SOL + SPL) as (symbol, balance_ui, mint_address).
pub fn get_all_token_balances() -> Vec<(String, f64, String)> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()
        .ok();
    let stdout = output
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let json: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    let mut result = Vec::new();
    if let Some(assets) = json["data"]["details"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|d| d["tokenAssets"].as_array())
    {
        for asset in assets {
            let symbol = asset["symbol"].as_str().unwrap_or("UNKNOWN").to_string();
            let balance = asset["balance"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let mint = asset["tokenAddress"].as_str().unwrap_or("").to_string();
            if balance > 0.0 {
                result.push((symbol, balance, mint));
            }
        }
    }
    result
}

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
/// to: Kamino KVault Program ID.
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
            "--biz-type",
            BIZ_TYPE,
            "--strategy",
            STRATEGY,
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

    if result.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        let msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        anyhow::bail!("onchainos contract-call returned error: {}", msg);
    }

    Ok(result)
}

/// Poll onchainos wallet history until the tx reaches SUCCESS or FAILED, or 60s timeout.
/// Returns Ok(()) on SUCCESS, Err on FAILED or timeout.
pub async fn wait_for_tx_solana(tx_hash: &str, wallet: &str) -> anyhow::Result<()> {
    let tx = tx_hash.to_string();
    let wlt = wallet.to_string();
    tokio::task::spawn_blocking(move || wait_for_tx_solana_sync(&tx, &wlt))
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking error: {}", e))?
}

fn wait_for_tx_solana_sync(tx_hash: &str, wallet: &str) -> anyhow::Result<()> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        if std::time::Instant::now() > deadline {
            anyhow::bail!("Timeout (60s) waiting for tx {} to confirm on-chain", tx_hash);
        }
        let output = Command::new("onchainos")
            .args([
                "wallet", "history",
                "--tx-hash", tx_hash,
                "--address", wallet,
                "--chain", "501",
            ])
            .output();
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                let entry = v["data"]
                    .as_array()
                    .and_then(|a| a.first())
                    .cloned()
                    .unwrap_or_else(|| v["data"].clone());
                match entry["txStatus"].as_str() {
                    Some("SUCCESS") => return Ok(()),
                    Some("FAILED") => {
                        let reason = entry["failReason"].as_str().unwrap_or("unknown");
                        anyhow::bail!("tx {} failed on-chain: {}", tx_hash, reason);
                    }
                    _ => {} // PENDING or not yet indexed — keep polling
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

/// Extract txHash from onchainos response.
/// Returns an error if no txHash is found.
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
