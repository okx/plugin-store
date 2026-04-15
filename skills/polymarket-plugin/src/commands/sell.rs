use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_sell_worst_price, get_balance_allowance, get_market_fee, get_orderbook,
    post_order, round_price, to_token_units, OrderBody,
    OrderRequest,
};
use crate::auth::ensure_credentials;
use crate::onchainos::{approve_ctf, get_wallet_address, is_ctf_approved_for_all};
use crate::series;
use crate::signing::{sign_order_via_onchainos, OrderParams};

use super::buy::{resolve_from_gamma, resolve_market_token};

/// Run the sell command.
///
/// market_id: condition_id (0x-prefixed), slug, or series ID (e.g. btc-5m). Optional when
///   token_id_fast is provided.
/// mode_override: optional one-time trading mode override ("eoa" or "proxy").
/// token_id_fast: skip all market resolution when token ID is known (from get-series output).
pub async fn run(
    market_id: Option<&str>,
    outcome: &str,
    shares: &str,
    price: Option<f64>,
    order_type: &str,
    auto_approve: bool,
    dry_run: bool,
    post_only: bool,
    expires: Option<u64>,
    mode_override: Option<&str>,
    token_id_fast: Option<&str>,
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

    // Geo check — hard fail before any live trading attempt.
    // Skipped for dry-run so users can preview orders regardless of region.
    if !dry_run {
        if let Some(geo_msg) = crate::api::check_clob_access(&client).await {
            bail!("{}", geo_msg);
        }
    }

    // ── Public API phase (no auth, runs for dry-run too) ─────────────────────

    let (condition_id, token_id, neg_risk, fee_rate_bps, book, signer_addr_opt) =
        if let Some(tid) = token_id_fast {
            // ── Fast path: token_id provided directly ──────────────────────────
            let book = get_orderbook(&client, tid).await?;
            let condition_id = book.market.clone()
                .ok_or_else(|| anyhow::anyhow!(
                    "Order book did not return a condition_id for token {}. \
                     Try using --market-id instead.", tid
                ))?;
            let neg_risk = book.neg_risk;
            let token_id = tid.to_string();

            let (fee_r, wallet_opt) = if dry_run {
                let fee = get_market_fee(&client, &condition_id).await.unwrap_or(0);
                (fee, None)
            } else {
                let (fee_res, wallet_res) = tokio::join!(
                    get_market_fee(&client, &condition_id),
                    get_wallet_address()
                );
                (fee_res.unwrap_or(0), Some(wallet_res?))
            };

            (condition_id, token_id, neg_risk, fee_r, book, wallet_opt)
        } else {
            let mid = market_id.ok_or_else(|| anyhow::anyhow!(
                "--market-id is required when --token-id is not provided"
            ))?;

            if series::is_series_id(mid) {
                // ── Series path ────────────────────────────────────────────────
                let gamma = series::resolve_to_market(&client, mid).await?;
                let (cid, tid, nr, fee) = resolve_from_gamma(&client, gamma, outcome).await?;

                let (book, wallet_opt) = if dry_run {
                    (get_orderbook(&client, &tid).await?, None)
                } else {
                    let (b, w) = tokio::join!(
                        get_orderbook(&client, &tid),
                        get_wallet_address()
                    );
                    (b?, Some(w?))
                };

                (cid, tid, nr, fee, book, wallet_opt)
            } else {
                // ── Standard path: slug or condition_id ────────────────────────
                let (cid, tid, nr, fee) = resolve_market_token(&client, mid, outcome).await?;

                let (book, wallet_opt) = if dry_run {
                    (get_orderbook(&client, &tid).await?, None)
                } else {
                    let (b, w) = tokio::join!(
                        get_orderbook(&client, &tid),
                        get_wallet_address()
                    );
                    (b?, Some(w?))
                };

                (cid, tid, nr, fee, book, wallet_opt)
            }
        };

    // Extract tick_size from the order book (avoids a separate get_tick_size call).
    let tick_size = book.tick_size.as_deref()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|&t| t > 0.0)
        .unwrap_or(0.01);

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
    let g2 = gcd(step_raw, 10_000);
    let step = step_raw / g2 * 10_000;

    let max_maker_raw = (share_amount * 1_000_000.0).floor() as u128;
    let maker_amount_raw = (max_maker_raw / step) * step;
    let taker_amount_raw = price_ticks * maker_amount_raw / tick_scale;

    // Guard: share amount too small to produce a valid order after GCD alignment.
    // This check fires BEFORE any approval tx is submitted.
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

    use crate::config::{Contracts, TradingMode};

    // Wallet address was pre-fetched in parallel with the order book (non-dry-run path).
    let signer_addr = signer_addr_opt.expect("signer_addr must be set in non-dry-run path");
    let creds = ensure_credentials(&client, &signer_addr).await?;

    // Resolve effective trading mode.
    let effective_mode = match mode_override {
        Some("proxy") => TradingMode::PolyProxy,
        Some("eoa")   => TradingMode::Eoa,
        _             => creds.mode.clone(),
    };

    let (maker_addr, sig_type) = match &effective_mode {
        TradingMode::PolyProxy => {
            let proxy = creds.proxy_wallet.as_ref().ok_or_else(|| anyhow::anyhow!(
                "POLY_PROXY mode requires a proxy wallet. \
                 Run `polymarket setup-proxy` to create one first."
            ))?.clone();
            eprintln!("[polymarket] Using POLY_PROXY mode — maker: {}", proxy);
            (proxy, 1u8)
        }
        TradingMode::Eoa => (signer_addr.clone(), 0u8),
    };

    // Check CTF token balance (from maker's address).
    // EOA mode: use CLOB API (reliable for EOA wallets).
    // POLY_PROXY mode: CLOB API returns 0 for proxy wallets regardless of actual balance;
    // skip the proxy pre-flight and let the CLOB server validate at order submission.
    // However, as a heuristic we check if the EOA holds the tokens — if so, warn the
    // user that they may have the wrong mode set (bought in EOA, now selling in proxy).
    let shares_needed_raw = to_token_units(share_amount);
    if effective_mode == TradingMode::Eoa {
        let token_balance = get_balance_allowance(&client, &maker_addr, &creds, "CONDITIONAL", Some(&token_id)).await?;
        let balance_raw = token_balance.balance.as_deref().unwrap_or("0").parse::<u64>().unwrap_or(0);

        if balance_raw < shares_needed_raw {
            // Check if the proxy wallet might hold these tokens and hint mode switch.
            let proxy_hint = crate::config::load_credentials()
                .ok()
                .flatten()
                .and_then(|c| c.proxy_wallet)
                .map(|proxy| format!(
                    " Your position tokens may be in the proxy wallet ({}). \
                     Switch modes with: polymarket switch-mode --mode proxy",
                    proxy
                ))
                .unwrap_or_default();
            bail!(
                "Insufficient token balance in EOA wallet: have {:.6} shares, need {:.6} shares.{}",
                balance_raw as f64 / 1_000_000.0,
                share_amount,
                proxy_hint
            );
        }
    } else {
        // Proxy mode: CLOB API can't verify proxy token balance directly.
        // Check EOA balance as a heuristic — if EOA holds enough tokens the user
        // likely bought in EOA mode and is now selling in proxy mode (wrong mode).
        if let Ok(eoa_bal) = get_balance_allowance(
            &client, &signer_addr, &creds, "CONDITIONAL", Some(&token_id)
        ).await {
            let eoa_raw = eoa_bal.balance.as_deref()
                .unwrap_or("0").parse::<u64>().unwrap_or(0);
            if eoa_raw >= shares_needed_raw {
                eprintln!(
                    "[polymarket] Warning: found {:.6} {} tokens in EOA wallet ({}) — \
                     your position may be in EOA, not proxy. \
                     If sell fails, switch modes with: polymarket switch-mode --mode eoa",
                    eoa_raw as f64 / 1_000_000.0,
                    outcome,
                    signer_addr
                );
            }
        }
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

    // EOA mode: verify POL balance for gas. Proxy mode uses relayer — no POL needed.
    if effective_mode == TradingMode::Eoa {
        const MIN_POL: f64 = 0.01;
        match crate::onchainos::get_pol_balance(&signer_addr).await {
            Ok(pol) if pol < MIN_POL => {
                bail!(
                    "Insufficient POL for gas: have {:.4} POL, need at least {} POL. \
                     Swap USDC to POL using `pancakeswap-v3 swap` or `okx-dex swap`, \
                     or switch to gasless POLY_PROXY mode with `polymarket setup-proxy`.",
                    pol, MIN_POL
                );
            }
            Err(e) => {
                eprintln!("[polymarket] Warning: could not verify POL balance ({}); proceeding.", e);
            }
            Ok(_) => {}
        }
    }

    // EOA mode: check and submit CTF setApprovalForAll if needed.
    // POLY_PROXY mode: no approval tx — relayer handles settlement through the proxy.
    if effective_mode == TradingMode::Eoa {
        let already_approved = if neg_risk {
            let ok1 = match is_ctf_approved_for_all(&signer_addr, Contracts::NEG_RISK_CTF_EXCHANGE).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[polymarket] Note: could not verify NEG_RISK_CTF_EXCHANGE approval ({}); will re-approve.", e);
                    false
                }
            };
            let ok2 = match is_ctf_approved_for_all(&signer_addr, Contracts::NEG_RISK_ADAPTER).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[polymarket] Note: could not verify NEG_RISK_ADAPTER approval ({}); will re-approve.", e);
                    false
                }
            };
            ok1 && ok2
        } else {
            match is_ctf_approved_for_all(&signer_addr, Contracts::CTF_EXCHANGE).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[polymarket] Note: could not verify CTF_EXCHANGE approval ({}); will re-approve.", e);
                    false
                }
            }
        };
        if !already_approved || auto_approve {
            let exchange_label = if neg_risk { "Neg Risk CTF Exchange" } else { "CTF Exchange" };
            eprintln!("[polymarket] Approving CTF tokens for {}...", exchange_label);
            let tx_hash = approve_ctf(neg_risk).await?;
            eprintln!("[polymarket] Approval tx: {}", tx_hash);
            eprintln!("[polymarket] Waiting for approval to confirm on-chain...");
            crate::onchainos::wait_for_tx_receipt(&tx_hash, 30).await?;
            eprintln!("[polymarket] Approval confirmed.");
        }
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
        signature_type: sig_type,
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
        signature_type: sig_type,
        signature,
    };

    let order_req = OrderRequest {
        order: order_body,
        owner: creds.api_key.clone(),
        order_type: effective_order_type.to_uppercase(),
        post_only,
    };

    // The order owner for L2 auth must always be the EOA (API key holder),
    // regardless of trading mode. In POLY_PROXY mode the maker field in the
    // order struct is the proxy, but the HTTP owner must match the API key.
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
