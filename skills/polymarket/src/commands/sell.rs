use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_sell_worst_price, get_balance_allowance, get_orderbook, get_tick_size, post_order,
    round_amount_down, round_price, round_size_down, to_token_units, OrderBody, OrderRequest,
};
use crate::auth::ensure_credentials;
use crate::config::{get_or_create_signing_key, signing_key_address};
use crate::onchainos::{approve_ctf, ensure_operator_approval, get_wallet_address};
use crate::signing::{sign_order, OrderParams};

use super::buy::resolve_market_token;

/// Run the sell command.
///
/// market_id: condition_id (0x-prefixed) or slug
/// outcome: "yes" or "no"
/// shares: number of token shares to sell (human-readable)
/// price: limit price in [0, 1], or None for market order (FOK)
pub async fn run(
    market_id: &str,
    outcome: &str,
    shares: &str,
    price: Option<f64>,
    order_type: &str,
    auto_approve: bool,
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "market_id": market_id,
                    "outcome": outcome,
                    "shares": shares,
                    "estimated_price": null,
                    "note": "dry-run: order not submitted"
                }
            })
        );
        return Ok(());
    }

    let client = Client::new();

    let signing_key = get_or_create_signing_key()?;
    let signer_addr = signing_key_address(&signing_key);
    let wallet_addr = get_wallet_address().await?;
    ensure_operator_approval(&wallet_addr, &signer_addr, false).await?;
    let creds = ensure_credentials(&client, &signing_key).await?;

    let (condition_id, token_id, neg_risk) = resolve_market_token(&client, market_id, outcome).await?;

    let tick_size = get_tick_size(&client, &token_id).await?;

    let share_amount: f64 = shares.parse().context("invalid shares amount")?;
    if share_amount <= 0.0 {
        bail!("shares must be positive");
    }

    // Determine price
    let limit_price = if let Some(p) = price {
        if p <= 0.0 || p >= 1.0 {
            bail!("price must be in range (0, 1)");
        }
        round_price(p, tick_size)
    } else {
        let book = get_orderbook(&client, &token_id).await?;
        compute_sell_worst_price(&book.bids, share_amount)
            .ok_or_else(|| anyhow::anyhow!("No bids available in the order book"))?
    };

    // Check CTF token balance
    let token_balance = get_balance_allowance(&client, &signer_addr, &creds, "CONDITIONAL", Some(&token_id)).await?;
    let balance_raw = token_balance.balance.as_deref().unwrap_or("0").parse::<u64>().unwrap_or(0);
    let shares_needed_raw = to_token_units(share_amount);

    if balance_raw < shares_needed_raw {
        bail!(
            "Insufficient token balance: have {} raw units ({:.6} shares), need {} raw units ({:.6} shares)",
            balance_raw,
            balance_raw as f64 / 1_000_000.0,
            shares_needed_raw,
            share_amount
        );
    }

    // Check CTF token allowance and auto-approve if needed
    let allowance_raw = token_balance.allowance.as_deref().unwrap_or("0").parse::<u64>().unwrap_or(0);
    if allowance_raw < shares_needed_raw || auto_approve {
        eprintln!("[polymarket] Approving CTF tokens for CTF Exchange...");
        let tx_hash = approve_ctf(neg_risk).await?;
        eprintln!("[polymarket] Approval tx: {}", tx_hash);
    }

    // Build order amounts (SELL)
    let rounded_shares = round_size_down(share_amount);
    let maker_amount_raw = to_token_units(rounded_shares); // shares to sell

    // taker_amount = shares * price (USDC.e to receive)
    let usdc_out = rounded_shares * limit_price;
    let rounded_usdc = round_amount_down(usdc_out, tick_size);
    let taker_amount_raw = to_token_units(rounded_usdc);

    let salt = rand_salt();

    let params = OrderParams {
        salt,
        maker: wallet_addr.clone(),
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw,
        taker_amount: taker_amount_raw,
        expiration: 0,
        nonce: 0,
        fee_rate_bps: 0,
        side: 1, // SELL
        signature_type: 0,
    };

    let signature = sign_order(&signing_key, &params, neg_risk)?;

    let order_body = OrderBody {
        salt: salt.to_string(),
        maker: wallet_addr.clone(),
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        expiration: "0".to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: "0".to_string(),
        side: "SELL".to_string(),
        signature_type: 0,
        signature,
    };

    let order_req = OrderRequest {
        order: order_body,
        owner: creds.api_key.clone(),
        order_type: order_type.to_uppercase(),
        post_only: false,
    };

    let resp = post_order(&client, &signer_addr, &creds, &order_req).await?;

    if resp.success != Some(true) {
        let msg = resp.error_msg.as_deref().unwrap_or("unknown error");
        bail!("Order placement failed: {}", msg);
    }

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "order_id": resp.order_id,
            "status": resp.status,
            "condition_id": condition_id,
            "outcome": outcome,
            "token_id": token_id,
            "side": "SELL",
            "order_type": order_type.to_uppercase(),
            "limit_price": limit_price,
            "shares": rounded_shares,
            "usdc_out": rounded_usdc,
            "maker_amount_raw": maker_amount_raw,
            "taker_amount_raw": taker_amount_raw,
            "tx_hashes": resp.tx_hashes,
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn rand_salt() -> u128 {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    u128::from_le_bytes(bytes)
}
