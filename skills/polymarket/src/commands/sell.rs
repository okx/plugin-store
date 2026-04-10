use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_sell_worst_price, get_balance_allowance, get_market_fee, get_orderbook, get_tick_size,
    post_order, round_amount_down, round_price, round_size_down, to_token_units, OrderBody,
    OrderRequest,
};
use crate::auth::ensure_credentials;
use crate::onchainos::{approve_ctf, get_wallet_address};
use crate::signing::{sign_order_via_onchainos, OrderParams};

use super::buy::resolve_market_token;

/// Run the sell command.
///
/// market_id: condition_id (0x-prefixed) or slug
/// outcome: outcome label, case-insensitive (e.g. "yes", "no", "trump")
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

    // onchainos wallet is the signer (approved operator of proxy wallet after polymarket.com onboarding)
    let signer_addr = get_wallet_address().await?;

    // Derive API credentials for the onchainos wallet
    let creds = ensure_credentials(&client, &signer_addr).await?;

    // EOA mode (signature_type=0): maker = signer = onchainos wallet.
    // No proxy wallet or polymarket.com onboarding required.
    let maker_addr = signer_addr.clone();

    let (condition_id, token_id, neg_risk) = resolve_market_token(&client, market_id, outcome).await?;

    let tick_size = get_tick_size(&client, &token_id).await?;
    let fee_rate_bps = get_market_fee(&client, &condition_id).await.unwrap_or(0);

    let share_amount: f64 = shares.parse().context("invalid shares amount")?;
    if share_amount <= 0.0 {
        bail!("shares must be positive");
    }

    // Determine price
    let limit_price = if let Some(p) = price {
        if p <= 0.0 || p >= 1.0 {
            bail!("price must be in range (0, 1)");
        }
        let rp = round_price(p, tick_size);
        if rp <= 0.0 || rp >= 1.0 {
            bail!("price {p} rounds to {rp} with tick size {tick_size} — out of range (0, 1)");
        }
        rp
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
    use crate::config::Contracts;
    let exchange_addr = Contracts::exchange_for(neg_risk);
    let allowance_raw = token_balance.allowance_for(exchange_addr);
    if allowance_raw < shares_needed_raw || auto_approve {
        eprintln!("[polymarket] Approving CTF tokens for CTF Exchange...");
        let tx_hash = approve_ctf(neg_risk).await?;
        eprintln!("[polymarket] Approval tx: {}", tx_hash);
    }

    // Build order amounts (SELL)
    let rounded_shares = round_size_down(share_amount);
    let maker_amount_raw = to_token_units(rounded_shares); // shares to sell

    let usdc_out = rounded_shares * limit_price;
    let rounded_usdc = round_amount_down(usdc_out, tick_size);
    let taker_amount_raw = to_token_units(rounded_usdc);

    let salt = rand_salt();

    let params = OrderParams {
        salt,
        maker: maker_addr.clone(),    // EOA mode: maker = signer = onchainos wallet
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw,
        taker_amount: taker_amount_raw,
        expiration: 0,
        nonce: 0,
        fee_rate_bps,
        side: 1, // SELL
        signature_type: 0,
    };

    let signature = sign_order_via_onchainos(&params, neg_risk).await?;

    let order_body = OrderBody {
        salt,  // serialized as JSON number per clob-client spec
        maker: maker_addr.clone(),
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        expiration: "0".to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: fee_rate_bps.to_string(),
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

/// Generate a random salt within JavaScript's safe integer range (< 2^53).
fn rand_salt() -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    u64::from_le_bytes(bytes) & 0x001F_FFFF_FFFF_FFFF
}
