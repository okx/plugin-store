use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_buy_worst_price, get_balance_allowance, get_clob_market, get_market_fee, get_orderbook,
    get_tick_size, post_order, round_price,
    OrderBody, OrderRequest,
};
use crate::auth::ensure_credentials;
use crate::onchainos::{approve_usdc, get_wallet_address};
use crate::signing::{sign_order_via_onchainos, OrderParams};

/// Run the buy command.
pub async fn run(
    market_id: &str,
    outcome: &str,
    amount: &str,
    price: Option<f64>,
    order_type: &str,
    auto_approve: bool,
    dry_run: bool,
    round_up: bool,
    post_only: bool,
    expires: Option<u64>,
) -> Result<()> {
    // Parse USDC amount early so we can enforce the minimum order size
    // check even on dry-run (the agent needs to know before placing).
    let usdc_amount: f64 = amount.parse().context("invalid amount")?;
    if usdc_amount <= 0.0 {
        bail!("amount must be positive");
    }

    // Validate --post-only / --expires up front (no network calls needed).
    // GTD requires an expiration; --expires auto-selects order_type GTD.
    // FOK is always a taker and incompatible with --post-only.
    if post_only && order_type.to_uppercase() == "FOK" {
        bail!("--post-only is incompatible with --order-type FOK: FOK orders are always takers");
    }
    if order_type.to_uppercase() == "GTD" && expires.is_none() {
        bail!("--order-type GTD requires --expires <unix_timestamp>");
    }
    let (expiration, effective_order_type) = if let Some(ts) = expires {
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

    // Resolve market (no auth required — public API)
    let (condition_id, token_id, neg_risk) =
        resolve_market_token(&client, market_id, outcome).await?;

    // Fetch the order book. min_order_size is enforced by the CLOB for resting limit orders
    // (limit price below the current best ask). The plugin pre-validates this below and
    // fires a clear error before submitting. Market (FOK) orders are exempt.
    let book = get_orderbook(&client, &token_id).await?;

    if dry_run {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "market_id": market_id,
                    "outcome": outcome,
                    "amount": amount,
                    "note": "dry-run: order not submitted"
                }
            })
        );
        return Ok(());
    }

    // onchainos wallet is the signer (approved operator of proxy wallet after polymarket.com onboarding)
    let signer_addr = get_wallet_address().await?;

    // Derive API credentials for the onchainos wallet
    let creds = ensure_credentials(&client, &signer_addr).await?;

    // EOA mode (signature_type=0): maker = signer = onchainos wallet.
    // No proxy wallet or polymarket.com onboarding required.
    let maker_addr = signer_addr.clone();

    // Get tick size and market fee rate
    let tick_size = get_tick_size(&client, &token_id).await?;
    let fee_rate_bps = get_market_fee(&client, &condition_id).await.unwrap_or(0);

    // Determine price (limit or market)
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
        compute_buy_worst_price(&book.asks, usdc_amount)
            .ok_or_else(|| anyhow::anyhow!("No asks available in the order book"))?
    };

    // Build order amounts using integer arithmetic to guarantee maker/taker == limit_price exactly.
    //
    // Polymarket requires:
    //   maker_raw (USDC, 6 dec) divisible by 10,000  → max 2 USDC decimal places
    //   taker_raw (shares, 6 dec) divisible by 100   → max 4 share decimal places
    //   maker_raw / taker_raw == limit_price exactly
    //
    // Express price as integer ticks: price_ticks = round(price / tick_size).
    // tick_scale = round(1 / tick_size) — e.g. 100 for tick=0.01, 1000 for tick=0.001.
    //   maker_raw = price_ticks × taker_raw / tick_scale
    // For maker_raw to be divisible by 10,000 and taker_raw to be divisible by 100:
    //   step = lcm(tick_scale × 10,000 / gcd(price_ticks, tick_scale × 10,000), 100)
    // We snap taker_raw DOWN to the nearest step, then maker_raw follows exactly.
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 { let t = b; b = a % b; a = t; }
        a
    }
    let tick_scale = (1.0 / tick_size).round() as u128; // 100 for tick=0.01, 1000 for tick=0.001
    let price_ticks = (limit_price / tick_size).round() as u128;
    let g = gcd(price_ticks, tick_scale * 10_000);
    let step_raw = tick_scale * 10_000 / g;
    let g2 = gcd(step_raw, 100);
    let step = step_raw / g2 * 100; // lcm(step_raw, 100)

    let max_taker_raw = (usdc_amount / limit_price * 1_000_000.0).floor() as u128;
    let mut taker_amount_raw = if round_up {
        // Ceiling: snap UP to the nearest valid step (may spend slightly more than requested)
        ((max_taker_raw + step - 1) / step) * step
    } else {
        // Floor: never spend more than requested
        (max_taker_raw / step) * step
    };
    let mut maker_amount_raw = price_ticks * taker_amount_raw / tick_scale;

    // Guard: amount too small to satisfy divisibility constraints — bail before approval.
    if taker_amount_raw == 0 || maker_amount_raw == 0 {
        let min_usdc = step as f64 / 1_000_000.0 * limit_price;
        bail!(
            "Amount too small: ${:.6} at price {:.4} rounds to 0 shares after divisibility \
             alignment. Minimum for this market/price is ~${:.6}. Pass --round-up to \
             automatically place the minimum amount instead.",
            usdc_amount, limit_price, min_usdc
        );
    }

    // Guard: resting orders below the CLOB min_order_size are rejected.
    // A resting order is a limit order priced below the current best ask (won't fill immediately).
    // Market orders (FOK, no --price) are exempt from this minimum.
    let min_order_size: f64 = book.min_order_size.as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    // best ask: asks are descending in the CLOB API, so .last() = lowest price = best ask for buyer
    let best_ask_float: Option<f64> = book.asks.last().and_then(|a| a.price.parse().ok());
    let is_resting = price.is_some() && best_ask_float.map_or(false, |ba| limit_price < ba);
    let computed_shares = taker_amount_raw as f64 / 1_000_000.0;
    if is_resting && min_order_size > 0.0 && computed_shares < min_order_size {
        if round_up {
            // Snap up to the market's minimum order size
            let min_taker_raw = (min_order_size * 1_000_000.0).ceil() as u128;
            taker_amount_raw = ((min_taker_raw + step - 1) / step) * step;
            maker_amount_raw = price_ticks * taker_amount_raw / tick_scale;
            eprintln!(
                "[polymarket] Note: amount rounded up to market minimum of {} shares for resting order.",
                taker_amount_raw as f64 / 1_000_000.0
            );
        } else {
            let min_usdc = min_order_size * limit_price;
            bail!(
                "Order too small: {:.2} shares at price {:.4} is below this market's minimum of \
                 {} shares (≈${:.2} required). Pass --round-up to place the minimum instead.",
                computed_shares, limit_price, min_order_size, min_usdc
            );
        }
    }

    // Notify if amount was adjusted from what was requested
    let actual_usdc = maker_amount_raw as f64 / 1_000_000.0;
    if round_up && actual_usdc > usdc_amount + 1e-6 {
        eprintln!(
            "[polymarket] Note: amount rounded up from ${:.6} to ${:.6} to satisfy \
             order divisibility constraints.",
            usdc_amount, actual_usdc
        );
    } else if !round_up && actual_usdc < usdc_amount - 1e-6 {
        eprintln!(
            "[polymarket] Note: amount adjusted from ${:.6} to ${:.6} to satisfy \
             order divisibility constraints.",
            usdc_amount, actual_usdc
        );
    }

    // Check USDC allowance and auto-approve if needed.
    // Use maker_amount_raw as the needed amount (accounts for any round-up).
    // For neg_risk markets the CLOB checks allowance on BOTH NEG_RISK_CTF_EXCHANGE and
    // NEG_RISK_ADAPTER — take the minimum so we re-approve if either is insufficient.
    use crate::config::Contracts;
    let allowance_info =
        get_balance_allowance(&client, &signer_addr, &creds, "COLLATERAL", None).await?;
    let allowance_raw = if neg_risk {
        let a_exchange = allowance_info.allowance_for(Contracts::NEG_RISK_CTF_EXCHANGE);
        let a_adapter  = allowance_info.allowance_for(Contracts::NEG_RISK_ADAPTER);
        a_exchange.min(a_adapter)
    } else {
        allowance_info.allowance_for(Contracts::CTF_EXCHANGE)
    };
    let usdc_needed_raw = maker_amount_raw as u64;

    if allowance_raw < usdc_needed_raw || auto_approve {
        eprintln!("[polymarket] Approving {:.6} USDC.e for CTF Exchange...", actual_usdc);
        let tx_hash = approve_usdc(neg_risk, usdc_needed_raw).await?;
        eprintln!("[polymarket] Approval tx: {}", tx_hash);
    }

    let salt = rand_salt();

    let params = OrderParams {
        salt,
        maker: maker_addr.clone(),    // EOA mode: maker = signer = onchainos wallet
        signer: signer_addr.clone(),
        taker: "0x0000000000000000000000000000000000000000".to_string(),
        token_id: token_id.clone(),
        maker_amount: maker_amount_raw as u64,
        taker_amount: taker_amount_raw as u64,
        expiration,
        nonce: 0,
        fee_rate_bps,
        side: 0, // BUY
        signature_type: 0, // EOA
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
        expiration: expiration.to_string(),
        nonce: "0".to_string(),
        fee_rate_bps: fee_rate_bps.to_string(),
        side: "BUY".to_string(),
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
            "side": "BUY",
            "order_type": effective_order_type.to_uppercase(),
            "limit_price": limit_price,
            "usdc_amount": actual_usdc,
            "usdc_requested": usdc_amount,
            "shares": taker_amount_raw as f64 / 1_000_000.0,
            "rounded_up": round_up && actual_usdc > usdc_amount + 1e-6,
            "post_only": post_only,
            "expires": if expiration > 0 { serde_json::Value::Number(expiration.into()) } else { serde_json::Value::Null },
            "tx_hashes": resp.tx_hashes,
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Resolve (condition_id, token_id, neg_risk) from a market_id and outcome string.
/// Supports any outcome label (e.g. "yes", "no", "trump", "republican", "option-a").
pub async fn resolve_market_token(
    client: &Client,
    market_id: &str,
    outcome: &str,
) -> Result<(String, String, bool)> {
    let outcome_lower = outcome.to_lowercase();
    if market_id.starts_with("0x") || market_id.starts_with("0X") {
        let market = get_clob_market(client, market_id).await?;
        let token = market
            .tokens
            .iter()
            .find(|t| t.outcome.to_lowercase() == outcome_lower)
            .ok_or_else(|| {
                let available: Vec<&str> = market.tokens.iter().map(|t| t.outcome.as_str()).collect();
                anyhow::anyhow!("Outcome '{}' not found. Available outcomes: {:?}", outcome, available)
            })?;
        Ok((market.condition_id.clone(), token.token_id.clone(), market.neg_risk))
    } else {
        let gamma = crate::api::get_gamma_market_by_slug(client, market_id).await?;
        let condition_id = gamma
            .condition_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No condition_id in Gamma market response"))?;
        let token_ids = gamma.token_ids();
        let outcomes = gamma.outcome_list();
        let idx = outcomes
            .iter()
            .position(|o| o.to_lowercase() == outcome_lower)
            .ok_or_else(|| {
                anyhow::anyhow!("Outcome '{}' not found. Available outcomes: {:?}", outcome, outcomes)
            })?;
        let token_id = token_ids
            .get(idx)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No token_id for outcome index {}", idx))?;
        Ok((condition_id, token_id, gamma.neg_risk))
    }
}

/// Generate a random salt within JavaScript's safe integer range (< 2^53).
/// The clob-client sends salt as a JSON number (Number.parseInt), so we must
/// ensure no precision loss on the server side.
fn rand_salt() -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    // Mask to 53 bits = 0x1FFFFFFFFFFFFF
    u64::from_le_bytes(bytes) & 0x001F_FFFF_FFFF_FFFF
}
