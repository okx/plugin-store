use anyhow::Result;
use reqwest::Client;

use crate::api::{cancel_all_orders, cancel_market_orders, cancel_order};
use crate::auth::ensure_credentials;
use crate::onchainos::get_wallet_address;

/// Cancel a single order by order ID.
pub async fn run_cancel_order(order_id: &str) -> Result<()> {
    let client = Client::new();
    let signer_addr = get_wallet_address().await?;
    let creds = ensure_credentials(&client, &signer_addr).await?;

    let resp = cancel_order(&client, &signer_addr, &creds, order_id).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": resp,
    }))?);
    Ok(())
}

/// Cancel all open orders for the authenticated user.
pub async fn run_cancel_all() -> Result<()> {
    let client = Client::new();
    let signer_addr = get_wallet_address().await?;
    let creds = ensure_credentials(&client, &signer_addr).await?;

    let resp = cancel_all_orders(&client, &signer_addr, &creds).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": resp,
    }))?);
    Ok(())
}

/// Cancel all orders for a specific market (by condition_id).
pub async fn run_cancel_market(condition_id: &str, token_id: Option<&str>) -> Result<()> {
    let client = Client::new();
    let signer_addr = get_wallet_address().await?;
    let creds = ensure_credentials(&client, &signer_addr).await?;

    let resp = cancel_market_orders(&client, &signer_addr, &creds, condition_id, token_id).await?;

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": resp,
    }))?);
    Ok(())
}
