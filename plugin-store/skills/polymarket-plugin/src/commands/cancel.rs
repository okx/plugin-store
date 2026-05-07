use anyhow::Result;
use reqwest::Client;

use crate::api::{cancel_all_orders, cancel_market_orders, cancel_order};
use crate::auth::ensure_credentials;
use crate::onchainos::get_wallet_address;

/// Resolve the CLOB auth address and credentials based on trading mode.
/// In DEPOSIT_WALLET mode, orders are indexed by the deposit wallet's API key,
/// so cancel calls must authenticate as the deposit wallet, not the EOA.
async fn resolve_cancel_auth(client: &Client) -> anyhow::Result<(String, crate::config::Credentials)> {
    let signer_addr = get_wallet_address().await?;
    let stored = crate::config::load_credentials_for(&signer_addr).ok().flatten();
    if let Some(ref c) = stored {
        if c.mode == crate::config::TradingMode::DepositWallet {
            if c.deposit_wallet.is_some() {
                // Cancel auth uses EOA creds — same as order placement (buy.rs:order_auth_addr = signer_addr).
                let creds = ensure_credentials(client, &signer_addr).await?;
                return Ok((signer_addr, creds));
            }
        }
    }
    let creds = ensure_credentials(client, &signer_addr).await?;
    Ok((signer_addr, creds))
}

/// Cancel a single order by order ID.
pub async fn run_cancel_order(order_id: &str) -> Result<()> {
    match run_cancel_order_inner(order_id).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("cancel"), None)); Ok(()) }
    }
}

async fn run_cancel_order_inner(order_id: &str) -> Result<()> {
    let client = Client::new();
    let (auth_addr, creds) = resolve_cancel_auth(&client).await?;

    let resp = cancel_order(&client, &auth_addr, &creds, order_id).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": resp,
    }))?);
    Ok(())
}

/// Cancel all open orders for the authenticated user.
pub async fn run_cancel_all() -> Result<()> {
    match run_cancel_all_inner().await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("cancel"), None)); Ok(()) }
    }
}

async fn run_cancel_all_inner() -> Result<()> {
    let client = Client::new();
    let (auth_addr, creds) = resolve_cancel_auth(&client).await?;

    let resp = cancel_all_orders(&client, &auth_addr, &creds).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": resp,
    }))?);
    Ok(())
}

/// Cancel all orders for a specific market (by condition_id).
pub async fn run_cancel_market(condition_id: &str, token_id: Option<&str>) -> Result<()> {
    match run_cancel_market_inner(condition_id, token_id).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("cancel"), None)); Ok(()) }
    }
}

async fn run_cancel_market_inner(condition_id: &str, token_id: Option<&str>) -> Result<()> {
    let client = Client::new();
    let (auth_addr, creds) = resolve_cancel_auth(&client).await?;

    let resp = cancel_market_orders(&client, &auth_addr, &creds, condition_id, token_id).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": resp,
    }))?);
    Ok(())
}
