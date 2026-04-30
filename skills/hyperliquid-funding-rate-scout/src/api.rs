use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

const HL_API: &str = "https://api.hyperliquid.xyz/info";

/// POST to the Hyperliquid info endpoint using curl.
/// This avoids any HTTP library dependency issues.
pub fn info_post(body: &Value) -> Result<Value> {
    let body_str = serde_json::to_string(body)?;

    let output = Command::new("curl")
        .args([
            "-s",
            "-X", "POST",
            HL_API,
            "-H", "Content-Type: application/json",
            "-d", &body_str,
            "--max-time", "15",
        ])
        .output()
        .context("Failed to run curl — is curl installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("curl failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        anyhow::bail!("Hyperliquid API returned empty response. Check network connection.");
    }

    serde_json::from_str(&stdout)
        .with_context(|| format!("Failed to parse API response as JSON: {}", &stdout[..stdout.len().min(200)]))
}

/// Fetch meta + asset contexts (funding rates, open interest, mark prices).
pub fn get_meta_and_asset_ctxs() -> Result<(Value, Vec<Value>)> {
    let resp = info_post(&serde_json::json!({ "type": "metaAndAssetCtxs" }))?;

    let arr = resp
        .as_array()
        .context("metaAndAssetCtxs: expected array")?
        .clone();

    if arr.len() < 2 {
        anyhow::bail!("metaAndAssetCtxs: expected [meta, ctxs] array");
    }

    let meta = arr[0].clone();
    let ctxs = arr[1]
        .as_array()
        .context("metaAndAssetCtxs: ctxs is not an array")?
        .clone();

    Ok((meta, ctxs))
}

/// Fetch all mid prices.
pub fn get_all_mids() -> Result<Value> {
    info_post(&serde_json::json!({ "type": "allMids" }))
}