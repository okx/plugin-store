use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_sell_worst_price, get_balance_allowance, get_market_fee, get_orderbook, get_tick_size,
    post_order, round_price, to_token_units, OrderBody,
    OrderRequest,
};
use crate::auth::ensure_credentials;
use crate::onchainos::{approve_ctf, get_wallet_address, is_ctf_approved_for_all};
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
    post_only: bool,
    expires: Option<u64>,
) -> Result<()> {
    // Parse shares and validate order flags up front (before any network calls).
    let share_amount: f64 = shares.parse().context("invalid shares amount")?;
    if share_amount <= 0.0 {
        bail!("shares must be positive");
    }

    if post_only && order_type.to_uppercase() == "FOK" {
        bail!("--post-only is incompatible with --order-type FOK: FOK orders are always takers");
    }
    if order_type.to_uppercase() == "GTD" && expires.is_none() {
        bail!("--order-type GTD requires --expires <unix_timestamp>");
    }
    let (expiration, mut effective_order_type) = if let Some(ts) = expires {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if ts < now + 90 {
            bail!("--expires must be at least 90 seconds in the future (got {ts}, now {now})");
        }
        (ts, "GTD")
    } else {
        (0, order_type)
    };

    let client = Client::new();

    // ── Public API phase (no auth, runs for dry-run too) ─────────────────────

    let (condition_id, token_id, neg_risk) = resolve_market_token(&client, market_id, outcome).await?;

    let tick_size = get_tick_size(&client, &token_id).await?;
    let fee_rate_bps = get_market_fee(&client, &condition_id).await.unwrap_or(0);

    // Determine price.
    let requested_price = price; // keep for adjustment warning
    let limit_price = if let Some(p) = price {
        if p <= 0.0 || p >= 1.0 {
            bail!("price must be in range (0, 1)");
        }
        let rp = round_price(p, tick_size);
        if rp <= 0.0 || rp >= 1.0 {
            bail!("price {p} rounds to {rp} with tick size {tick_size} — out of range (0, 1)");
        }
        // Warn if price was adjusted to satisfy tick size constraint.
        if (rp - p).abs() > 1e-9 {
            eprintln!(
                "[polymarket] Note: price adjusted from {:.6} to {:.6} to satisfy tick size constraint ({}).",
                p, rp, tick_size
            );
        }
        rp
    } else {
        let book = get_orderbook(&client, &token_id).await?;
        if let Some(p) = compute_sell_worst_price(&book.bids, share_amount) {
            p
        } else {
            // No bids — convert market order to GTC limit at last trade price.
            let fallback = book.last_trade_price
                .as_deref()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|&p| p > 0.0 && p < 1.0)
                .map(|p| round_price(p, tick_size));
            let fp = fallback.ok_or_else(|| anyhow::anyhow!(
                "No bids in the order book and no last trade price available. \
                 Pass --price to place a limit order manually."
            ))?;
            effective_order_type = "GTC";
            eprintln!(
                "[polymarket] No bids in order book — converting market order to GTC limit at \
                 last trade price {:.4}. Pass --price to set a specific price.",
                fp
            );
            fp
        }
    };

    // Build order amounts (SELL) using GCD-based integer arithmetic.
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 { let t = b; b = a % b; a = t; }
        a
    }
    let tick_scale = (1.0 / tick_size).round() as u128;
    let price_ticks = (limit_price / tick_size).round() as u128;
    let g = gcd(price_ticks, tick_scale * 10_000);
    let step_raw = tick_scale * 10_000 / g;
    let g2 = gcd(step_raw, 100);
    let step = step_raw / g2 * 100;

    let max_maker_raw = (share_amount * 1_000_000.0).floor() as u128;
    let maker_amount_raw = (max_maker_raw / step) * step;
    let taker_amount_raw = price_ticks * maker_amount_raw / tick_scale;

    // Guard: share amount too small to produce a valid order after GCD alignment.
    // This check fires BEFORE any approval tx is submitted (fixes M1).
    if maker_amount_raw == 0 || taker_amount_raw == 0 {
        bail!(
            "Amount too small: {:.6} shares at price {:.4} rounds to 0 after divisibility \
             alignment. Minimum for this market/price is ~{:.6} shares. \
             Consider using a larger amount.",
            share_amount, limit_price, step as f64 / 1_000_000.0
        );
    }

    let actual_shares = maker_amount_raw as f64 / 1_000_000.0;

    // ── Dry-run exit — full projected order fields ────────────────────────────
    if dry_run {
        // Include price adjustment info in dry-run if applicable.
        let price_adjusted = requested_price.map_or(false, |p| (limit_price - p).abs() > 1e-9);
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "market_id": market_id,
                    "condition_id": condition_id,
                    "outcome": outcome,
                    "token_id": token_id,
                    "side": "SELL",
                    "order_type": effective_order_type.to_uppercase(),
                    "limit_price": limit_price,
                    "limit_price_requested": requested_price,
                    "price_adjusted": price_adjusted,
                    "shares": actual_shares,
                    "shares_requested": share_amount,
                    "usdc_out": taker_amount_raw as f64 / 1_000_000.0,
                    "fee_rate_bps": fee_rate_bps,
                    "post_only": post_only,
                    "expires": if expiration > 0 { serde_json::Value::Number(expiration.into()) } else { serde_json::Value::Null },
                    "note": "dry-run: order not submitted"
                }
            })
        );
        return Ok(());
    }

    // ── Auth phase ────────────────────────────────────────────────────────────

    let signer_addr = get_wallet_address().await?;
    let creds = ensure_credentials(&client, &signer_addr).await?;
    let maker_addr = signer_addr.clone();

    // Check CTF token balance.
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

    // Warn if GCD alignment reduced the share amount.
    if actual_shares < share_amount - 1e-9 {
        eprintln!(
            "[polymarket] Note: share amount adjusted from {:.6} to {:.6} to satisfy \
             order divisibility constraints. The remaining {:.6} shares cannot be included \
             in this order.",
            share_amount, actual_shares, share_amount - actual_shares
        );
    }

    // Check CTF token approval via on-chain isApprovedForAll (ERC-1155).
    use crate::config::Contracts;
    let already_approved = if neg_risk {
        let ok1 = match is_ctf_approved_for_all(&signer_addr, Contracts::NEG_RISK_CTF_EXCHANGE).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[polymarket] Note: could not verify NEG_RISK_CTF_EXCHANGE approval ({}); will re-approve (setApprovalForAll is idempotent).", e);
                false
            }
        };
        let ok2 = match is_ctf_approved_for_all(&signer_addr, Contracts::NEG_RISK_ADAPTER).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[polymarket] Note: could not verify NEG_RISK_ADAPTER approval ({}); will re-approve (setApprovalForAll is idempotent).", e);
                false
            }
        };
        ok1 && ok2
    } else {
        match is_ctf_approved_for_all(&signer_addr, Contracts::CTF_EXCHANGE).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[polymarket] Note: could not verify CTF_EXCHANGE approval ({}); will re-approve (setApprovalForAll is idempotent).", e);
                false
            }
        }
    };
    if !already_approved || auto_approve {
        let exchange_label = if neg_risk { "Neg Risk CTF Exchange" } else { "CTF Exchange" };
        eprintln!("[polymarket] Approving CTF tokens for {}...", exchange_label);
        let tx_hash = approve_ctf(neg_risk).await?;
        eprintln!("[polymarket] Approval tx: {}", tx_hash);
    }

    let salt = rand_salt();

    let params = OrderParams {
        salt,
        maker: maker_addr.clone(),
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw as u64,
        taker_amount: taker_amount_raw as u64,
        expiration,
        nonce: 0,
        fee_rate_bps,
        side: 1, // SELL
        signature_type: 0,
    };

    let signature = sign_order_via_onchainos(&params, neg_risk).await?;

    let order_body = OrderBody {
        salt,
        maker: maker_addr.clone(),
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw.to_string(),
        taker_amount: taker_amount_raw.to_string(),
        expiration: expiration.to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: fee_rate_bps.to_string(),
        side: "SELL".to_string(),
        signature_type: 0,
        signature,
    };

    let order_req = OrderRequest {
        order: order_body,
        owner: creds.api_key.clone(),
        order_type: effective_order_type.to_uppercase(),
        post_only,
    };

    let resp = post_order(&client, &signer_addr, &creds, &order_req).await?;

    if resp.success != Some(true) {
        let msg = resp.error_msg.as_deref().unwrap_or("unknown error");
        if msg.to_uppercase().contains("INVALID_ORDER_MIN_SIZE") {
            bail!(
                "Order rejected by CLOB: amount is below this market's minimum order size. \
                 Try a larger amount."
            );
        }
        let msg_upper = msg.to_uppercase();
        if msg_upper.contains("NOT AUTHORIZED") || msg_upper.contains("UNAUTHORIZED") {
            let _ = crate::config::clear_credentials();
            bail!(
                "Order rejected: credentials are stale or invalid ({}). \
                 Cached credentials cleared — run the command again to re-derive.",
                msg
            );
        }
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
            "order_type": effective_order_type.to_uppercase(),
            "limit_price": limit_price,
            "shares": maker_amount_raw as f64 / 1_000_000.0,
            "usdc_out": taker_amount_raw as f64 / 1_000_000.0,
            "maker_amount_raw": maker_amount_raw,
            "taker_amount_raw": taker_amount_raw,
            "post_only": post_only,
            "expires": if expiration > 0 { serde_json::Value::Number(expiration.into()) } else { serde_json::Value::Null },
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
