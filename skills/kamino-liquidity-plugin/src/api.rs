//! Kamino Liquidity (KVault) REST API client
//!
//! Base URL: https://api.kamino.finance
//!
//! Verified endpoints:
//!   GET  /kvaults/vaults                          → list all kvaults
//!   GET  /kvaults/users/{wallet}/positions         → user share positions
//!   POST /ktx/kvault/deposit   body: {kvault, wallet, amount}  → {transaction: base64}
//!   POST /ktx/kvault/withdraw  body: {kvault, wallet, amount}  → {transaction: base64}
//!
//! Field names verified by live testing 2026-04-05:
//!   - deposit/withdraw body uses "kvault" (not "vault", not "strategy")
//!   - deposit/withdraw body uses "wallet" (not "owner")
//!   - deposit/withdraw body uses "amount" (not "depositAmount", "sharesAmount")
//!   - amount is in UI units: "0.001" SOL = 0.001 SOL (not 1000000 lamports)

use anyhow::Result;
use serde_json::Value;

use crate::config::API_BASE;

/// Fetch all Kamino KVaults.
/// GET /kvaults/vaults
/// Returns array of vault objects with address, state, programId fields.
pub async fn get_vaults() -> Result<Value> {
    let url = format!("{}/kvaults/vaults", API_BASE);
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let data: Value = resp.json().await?;
    Ok(data)
}

/// Fetch user KVault positions (share balances).
/// GET /kvaults/users/{wallet}/positions
/// Returns array of {vault, sharesAmount, tokenAmount} or empty array [].
pub async fn get_user_positions(wallet: &str) -> Result<Value> {
    let url = format!("{}/kvaults/users/{}/positions", API_BASE, wallet);
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let data: Value = resp.json().await?;
    Ok(data)
}

/// Build a deposit transaction for a KVault.
/// POST /ktx/kvault/deposit
/// Body: { kvault, wallet, amount }  — amount in UI units (e.g. "0.001" SOL)
/// Returns base64-encoded serialized Solana transaction.
pub async fn build_deposit_tx(vault: &str, wallet: &str, amount: &str) -> Result<String> {
    let url = format!("{}/ktx/kvault/deposit", API_BASE);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "kvault": vault,
        "wallet": wallet,
        "amount": amount
    });
    let resp = client.post(&url).json(&body).send().await?;
    let data: Value = resp.json().await?;
    if let Some(tx) = data["transaction"].as_str() {
        Ok(tx.to_string())
    } else {
        anyhow::bail!(
            "Kamino API deposit error: {}",
            data["message"].as_str().unwrap_or(&data.to_string())
        )
    }
}

/// Build a withdrawal transaction for a KVault.
/// POST /ktx/kvault/withdraw
/// Body: { kvault, wallet, amount }  — amount = shares to redeem (UI units)
/// Returns base64-encoded serialized Solana transaction.
pub async fn build_withdraw_tx(vault: &str, wallet: &str, amount: &str) -> Result<String> {
    let url = format!("{}/ktx/kvault/withdraw", API_BASE);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "kvault": vault,
        "wallet": wallet,
        "amount": amount
    });
    let resp = client.post(&url).json(&body).send().await?;
    let data: Value = resp.json().await?;
    if let Some(tx) = data["transaction"].as_str() {
        Ok(tx.to_string())
    } else {
        anyhow::bail!(
            "Kamino API withdraw error: {}",
            data["message"].as_str().unwrap_or(&data.to_string())
        )
    }
}
