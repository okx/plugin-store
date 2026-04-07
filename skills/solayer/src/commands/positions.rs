use crate::config::{SOLANA_RPC, SOLAYER_API_BASE, SSOL_MINT};
use serde_json::Value;
use std::process::Command;

/// Get the user's sSOL balance and its SOL/USD equivalent.
pub async fn execute() -> anyhow::Result<Value> {
    // 1. Get wallet address from onchainos
    let wallet = get_wallet_address()?;

    // 2. Query on-chain sSOL token account balance
    let ssol_balance = get_ssol_balance(&wallet).await?;

    // 3. Get exchange rate from Solayer API
    let (ssol_to_sol, apy) = get_rates().await?;

    let sol_value = ssol_balance * ssol_to_sol;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "wallet": wallet,
            "ssol_balance": ssol_balance,
            "ssol_mint": SSOL_MINT,
            "sol_value": sol_value,
            "ssol_to_sol_rate": ssol_to_sol,
            "apy_percent": apy
        }
    });
    Ok(result)
}

fn get_wallet_address() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {}", e))?;

    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    if let Some(addr) = json["data"]["address"].as_str() {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("Could not resolve wallet address from onchainos")
}

async fn get_ssol_balance(wallet: &str) -> anyhow::Result<f64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet,
            {"mint": SSOL_MINT},
            {"encoding": "jsonParsed"}
        ]
    });

    let resp = client
        .post(SOLANA_RPC)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query sSOL balance: {}", e))?;

    let json: Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse RPC response: {}", e))?;

    let accounts = json["result"]["value"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if accounts.is_empty() {
        return Ok(0.0);
    }

    // Sum all sSOL token accounts (normally just one)
    let mut total = 0.0f64;
    for account in &accounts {
        let ui_amount = account["account"]["data"]["parsed"]["info"]["tokenAmount"]["uiAmount"]
            .as_f64()
            .unwrap_or(0.0);
        total += ui_amount;
    }
    Ok(total)
}

async fn get_rates() -> anyhow::Result<(f64, f64)> {
    let url = format!("{}/info", SOLAYER_API_BASE);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch Solayer info: {}", e))?;

    let json: Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse rates: {}", e))?;

    let ssol_to_sol = json["ssol_to_sol"].as_f64().unwrap_or(1.0);
    let apy = json["apy"].as_f64().unwrap_or(0.0);
    Ok((ssol_to_sol, apy))
}
