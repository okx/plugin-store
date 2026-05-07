use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_sell_worst_price, get_balance_allowance, get_clob_version, get_market_fee,
    get_orderbook, post_order, round_price, to_token_units, OrderBody, OrderBodyV2,
    OrderRequest, OrderRequestV2,
};
use crate::auth::ensure_credentials;
use crate::config::OrderVersion;
use crate::onchainos::{get_wallet_address, get_pusd_balance, is_ctf_approved_for_all};
use crate::series;
use crate::signing::{sign_order_v2_via_onchainos, sign_order_v2_poly1271_via_onchainos, sign_order_via_onchainos, OrderParams,
    OrderParamsV2, BYTES32_ZERO};

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
    strategy_id: Option<&str>,
) -> Result<()> {
    match run_inner(
        market_id, outcome, shares, price, order_type, auto_approve, dry_run,
        post_only, expires, mode_override, token_id_fast, strategy_id,
    ).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("sell"), None)); Ok(()) }
    }
}

async fn run_inner(
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
    strategy_id: Option<&str>,
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

    // Build order amounts (SELL) using integer arithmetic.
    //
    // Constraint: taker_amount_raw = price_ticks × maker_amount_raw / tick_scale
    // must be a non-negative integer (USDC in millionths).
    //
    // Align to whole shares (1_000_000 raw) as the minimum step — same logic as buy.
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 { let t = b; b = a % b; a = t; }
        a
    }
    let tick_scale = (1.0 / tick_size).round() as u128;
    let price_ticks = (limit_price / tick_size).round() as u128;
    const SHARE_RAW: u128 = 1_000_000;
    let g = gcd((price_ticks * SHARE_RAW) % tick_scale.max(1), tick_scale.max(1));
    let shares_per_step = tick_scale.max(1) / g.max(1);
    let step = shares_per_step * SHARE_RAW;

    let max_maker_raw = (share_amount * 1_000_000.0).floor() as u128;
    let mut maker_amount_raw = (max_maker_raw / step) * step;
    let mut taker_amount_raw = price_ticks * maker_amount_raw / tick_scale;

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

    // Fetch CLOB version in parallel with credentials.
    let clob_version_raw = get_clob_version(&client).await?;
    let clob_version = if clob_version_raw == 2 { OrderVersion::V2 } else { OrderVersion::V1 };

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
        TradingMode::DepositWallet => {
            let dw = creds.deposit_wallet.as_ref().ok_or_else(|| anyhow::anyhow!(
                "DEPOSIT_WALLET mode requires a deposit wallet. \
                 Run `polymarket setup-deposit-wallet` to create one first."
            ))?.clone();
            eprintln!("[polymarket] Using DEPOSIT_WALLET mode — maker: {}", dw);
            (dw, 3u8) // POLY_1271
        }
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
    //
    // V2 migration: V2 uses a new exchange contract address for CTF approval.
    // If the user approved V1 exchange but not V2, the V2 exchange will be approved here.
    if effective_mode == TradingMode::Eoa {
        let exchange_addr = Contracts::exchange(clob_version, neg_risk);
        let already_approved = if neg_risk {
            let ok1 = match is_ctf_approved_for_all(&signer_addr, exchange_addr).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[polymarket] Note: could not verify exchange approval ({}); will re-approve.", e);
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
            match is_ctf_approved_for_all(&signer_addr, exchange_addr).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[polymarket] Note: could not verify exchange approval ({}); will re-approve.", e);
                    false
                }
            }
        };
        if !already_approved || auto_approve {
            let version_label = if clob_version == OrderVersion::V2 { " V2" } else { "" };
            let exchange_label = if neg_risk {
                format!("Neg Risk CTF Exchange{}", version_label)
            } else {
                format!("CTF Exchange{}", version_label)
            };
            eprintln!("[polymarket] Approving CTF tokens for {}...", exchange_label);
            let tx_hash = approve_ctf_versioned(neg_risk, clob_version).await?;
            eprintln!("[polymarket] Approval tx: {}", tx_hash);
            eprintln!("[polymarket] Waiting for approval to confirm on-chain...");
            crate::onchainos::wait_for_tx_receipt(&tx_hash, 30).await?;
            eprintln!("[polymarket] Approval confirmed.");
        }
    }

    let salt = rand_salt();

    // Sign and submit the order using the correct version's struct and exchange contract.
    let resp = match clob_version {
        OrderVersion::V2 => {
            // V2 CLOB amount precision constraints for SELL orders:
            //   maker (shares): max 5 decimal places → divisible by 10 in millionths
            //   taker (USDC):   max 2 decimal places → divisible by 10,000 in millionths
            //
            // IMPORTANT: taker = price_ticks * maker / tick_scale. The CLOB validates that
            // taker/maker equals a valid tick price. Rounding taker independently breaks the
            // price ratio — the CLOB rejects with "breaks minimum tick size rule".
            //
            // Correct approach: find the combined step for maker such that
            //   (a) price_ticks * maker / tick_scale is divisible by V2_USDC_STEP (taker 2dp), AND
            //   (b) maker is divisible by the GCD alignment step (price ratio integrity)
            //
            // min_maker_for_usdc = (tick_scale * V2_USDC_STEP) / gcd(price_ticks, tick_scale * V2_USDC_STEP)
            // combined_step = lcm(step, min_maker_for_usdc)
            const V2_USDC_STEP: u128 = 10_000;

            fn gcd_v2(a: u128, b: u128) -> u128 { if b == 0 { a } else { gcd_v2(b, a % b) } }

            let combined_step = if tick_scale > 0 && price_ticks > 0 {
                let g = gcd_v2(price_ticks, tick_scale.saturating_mul(V2_USDC_STEP));
                let min_maker_for_usdc = tick_scale.saturating_mul(V2_USDC_STEP) / g;
                let g2 = gcd_v2(step, min_maker_for_usdc);
                step / g2 * min_maker_for_usdc  // lcm(step, min_maker_for_usdc)
            } else {
                step
            };

            // Re-align from max_maker_raw using combined_step (combined_step ≥ step,
            // so this can only reduce maker further from the GCD-aligned value).
            maker_amount_raw = (max_maker_raw / combined_step) * combined_step;
            taker_amount_raw = price_ticks * maker_amount_raw / tick_scale;

            if maker_amount_raw == 0 || taker_amount_raw == 0 {
                anyhow::bail!(
                    "Amount too small for V2 precision: {:.6} shares at price {:.4} \
                     rounds to 0 after combined GCD + USDC-precision alignment. \
                     Minimum is ~{:.6} shares. Try a larger amount.",
                    share_amount, limit_price, combined_step as f64 / 1_000_000.0
                );
            }

            let v2_actual_shares = maker_amount_raw as f64 / 1_000_000.0;
            if v2_actual_shares < actual_shares - 1e-9 {
                eprintln!(
                    "[polymarket] V2 CLOB precision further reduced shares from {:.6} to {:.6} \
                     to ensure USDC payout is a valid 2dp amount.",
                    actual_shares, v2_actual_shares
                );
            }

            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            // For POLY_1271 (DepositWallet mode):
            //   maker = signer = deposit_wallet.
            //   CLOB maps api_key → deposit_wallet (via sync_balance_allowance_deposit_wallet).
            //   For sig_type=3: CLOB validates order.signer == deposit_wallet_of(API_KEY).
            //   deposit_wallet.isValidSignature(hash, ecdsa_sig_by_EOA) performs on-chain verification.
            // For EOA/PolyProxy: signer = EOA.
            let order_signer = if effective_mode == TradingMode::DepositWallet {
                maker_addr.clone() // deposit_wallet = ERC-1271 verifier
            } else {
                signer_addr.clone()
            };
            let order_creds = creds.clone();
            let order_auth_addr = signer_addr.clone();

            let params = OrderParamsV2 {
                salt,
                maker: maker_addr.clone(),
                signer: order_signer.clone(),
                token_id: token_id.clone(),
                maker_amount: maker_amount_raw as u64,
                taker_amount: taker_amount_raw as u64,
                side: 1, // SELL
                signature_type: sig_type,
                timestamp_ms,
                metadata: BYTES32_ZERO.to_string(),
                builder: BYTES32_ZERO.to_string(),
            };
            let signature = if effective_mode == TradingMode::DepositWallet {
                sign_order_v2_poly1271_via_onchainos(&params, neg_risk).await?
            } else {
                sign_order_v2_via_onchainos(&params, neg_risk).await?
            };
            let order_body = OrderBodyV2 {
                salt,
                maker: maker_addr.clone(),
                signer: order_signer.clone(),
                token_id: token_id.clone(),
                maker_amount: maker_amount_raw.to_string(),
                taker_amount: taker_amount_raw.to_string(),
                side: "SELL".to_string(),
                signature_type: sig_type,
                timestamp: timestamp_ms.to_string(),
                metadata: BYTES32_ZERO.to_string(),
                builder: BYTES32_ZERO.to_string(),
                signature,
            };
            let order_req = OrderRequestV2 {
                order: order_body,
                owner: order_creds.api_key.clone(),
                order_type: effective_order_type.to_uppercase(),
                post_only,
                expiration: if expiration > 0 { expiration.to_string() } else { String::new() },
            };
            post_order(&client, &order_auth_addr, &order_creds, &order_req).await?
        }
        OrderVersion::V1 => {
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
            post_order(&client, &signer_addr, &creds, &order_req).await?
        }
    };

    if resp.success != Some(true) {
        let msg = resp.error_msg.as_deref().unwrap_or("unknown error");
        let msg_lower = msg.to_lowercase();

        // ── Deposit wallet migration (V2 maker allowlist) ─────────────────────
        if msg_lower.contains("maker address not allowed") || msg_lower.contains("deposit wallet") {
            let pusd = get_pusd_balance(&maker_addr).await.unwrap_or(0.0);
            let mode_str = match &effective_mode {
                TradingMode::Eoa => "eoa",
                TradingMode::PolyProxy => "proxy",
                TradingMode::DepositWallet => "deposit_wallet",
            };
            let transfer_step = if pusd > 0.0 {
                format!("3. Transfer {:.2} pUSD from {} to the deposit_wallet address", pusd, maker_addr)
            } else {
                "3. Fund the deposit wallet with pUSD (transfer from your source of funds)".to_string()
            };
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "Deposit wallet required — V2 exchange does not accept this maker address.",
                "migration_required": true,
                "migration": {
                    "current_mode": mode_str,
                    "trading_address": maker_addr,
                    "pusd_at_trading_address": pusd,
                    "next_steps": [
                        "1. Run: polymarket setup-deposit-wallet",
                        "2. Note the deposit_wallet address in the output",
                        transfer_step,
                        "4. Retry your order — plugin will automatically use deposit wallet mode"
                    ]
                }
            })).unwrap_or_default());
            return Ok(());
        }

        if msg.to_uppercase().contains("INVALID_ORDER_MIN_SIZE") {
            bail!(
                "Order rejected by CLOB: amount is below this market's minimum order size. \
                 Try a larger amount."
            );
        }
        let msg_upper = msg.to_uppercase();
        if msg_upper.contains("NOT AUTHORIZED") || msg_upper.contains("UNAUTHORIZED") {
            let _ = crate::config::clear_credentials_for(&signer_addr);
            bail!(
                "Order rejected: credentials are stale or invalid ({}). \
                 Cached credentials cleared for {} — run the command again to re-derive.",
                msg, &signer_addr[..std::cmp::min(10, signer_addr.len())]
            );
        }
        if msg_upper.contains("ORDER_VERSION_MISMATCH") || msg_upper.contains("VERSION_MISMATCH") {
            bail!(
                "Order rejected: CLOB version mismatch (server reported: {}). \
                 The server may have just switched to a different order version. \
                 Run the command again to re-detect the current version.",
                msg
            );
        }
        bail!("Order placement failed: {}", msg);
    }

    let shares_filled = maker_amount_raw as f64 / 1_000_000.0;
    if let Some(sid) = strategy_id.filter(|s| !s.is_empty()) {
        let ts_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let report_payload = serde_json::json!({
            "wallet": signer_addr,
            "proxyAddress": creds.proxy_wallet.as_deref().unwrap_or(""),
            "order_id": resp.order_id.clone().unwrap_or_default(),
            "tx_hashes": resp.tx_hashes,
            "market_id": condition_id,
            "asset_id": token_id,
            "side": "SELL",
            "amount": format!("{}", shares_filled),
            "symbol": "USDC.e",
            "price": format!("{}", limit_price),
            "timestamp": ts_now,
            "strategy_id": sid,
            "plugin_name": "polymarket-plugin",
        });
        if let Err(e) = crate::onchainos::report_plugin_info(&report_payload).await {
            eprintln!("[polymarket] Warning: report-plugin-info failed: {}", e);
        }
    }

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "market_id": market_id,
            "order_id": resp.order_id,
            "status": resp.status,
            "condition_id": condition_id,
            "outcome": outcome,
            "token_id": token_id,
            "side": "SELL",
            "order_type": effective_order_type.to_uppercase(),
            "limit_price": limit_price,
            "shares": shares_filled,
            "usdc_out": taker_amount_raw as f64 / 1_000_000.0,
            "fee_rate_bps": fee_rate_bps,
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

/// Approve CTF tokens (setApprovalForAll) for the correct exchange contract based on CLOB version.
///
/// V2 migration: V2 introduces new exchange contract addresses. Users who already
/// approved V1 contracts will get an automatic V2 approval on their first V2 sell.
async fn approve_ctf_versioned(neg_risk: bool, version: OrderVersion) -> anyhow::Result<String> {
    use crate::config::Contracts;
    use crate::onchainos::ctf_set_approval_for_all;

    let ctf = Contracts::CTF;
    let exchange_addr = Contracts::exchange(version, neg_risk);

    if neg_risk {
        ctf_set_approval_for_all(ctf, exchange_addr).await?;
        ctf_set_approval_for_all(ctf, Contracts::NEG_RISK_ADAPTER).await
    } else {
        ctf_set_approval_for_all(ctf, exchange_addr).await
    }
}
