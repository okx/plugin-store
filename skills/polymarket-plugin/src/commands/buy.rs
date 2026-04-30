use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::api::{
    compute_buy_worst_price, get_clob_market, get_clob_version,
    get_market_fee, get_orderbook, post_order, round_price, OrderBody, OrderBodyV2,
    OrderRequest, OrderRequestV2,
};
use crate::auth::ensure_credentials;
use crate::config::OrderVersion;
use crate::onchainos::{approve_timeout_secs, get_pusd_balance, get_usdc_balance, get_wallet_address,
    proxy_pusd_approve, proxy_wrap_usdc_to_pusd, wait_for_tx_receipt, wrap_usdc_to_pusd};
use crate::series;
use crate::signing::{sign_order_v2_via_onchainos, sign_order_via_onchainos, OrderParams,
    OrderParamsV2, BYTES32_ZERO};


/// Run the buy command.
///
/// market_id: condition_id (0x-prefixed), slug, or series ID (e.g. btc-5m). Optional when
///   token_id_fast is provided.
/// mode_override: optional one-time trading mode override ("eoa" or "proxy").
///   Does not persist — use `switch-mode` to change the default.
/// token_id_fast: skip all market resolution when token ID is known (from get-series output).
pub async fn run(
    market_id: Option<&str>,
    outcome: &str,
    amount: &str,
    price: Option<f64>,
    order_type: &str,
    auto_approve: bool,
    dry_run: bool,
    round_up: bool,
    post_only: bool,
    expires: Option<u64>,
    mode_override: Option<&str>,
    token_id_fast: Option<&str>,
    strategy_id: Option<&str>,
) -> Result<()> {
    match run_inner(
        market_id, outcome, amount, price, order_type, auto_approve, dry_run,
        round_up, post_only, expires, mode_override, token_id_fast, strategy_id,
    ).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("buy"), None)); Ok(()) }
    }
}

async fn run_inner(
    market_id: Option<&str>,
    outcome: &str,
    amount: &str,
    price: Option<f64>,
    order_type: &str,
    auto_approve: bool,
    dry_run: bool,
    round_up: bool,
    post_only: bool,
    expires: Option<u64>,
    mode_override: Option<&str>,
    token_id_fast: Option<&str>,
    strategy_id: Option<&str>,
) -> Result<()> {
    // Parse USDC amount early so we can enforce the minimum order size
    // check even on dry-run (the agent needs to know before placing).
    let usdc_amount: f64 = amount.parse().context("invalid amount")?;
    if usdc_amount <= 0.0 {
        bail!("amount must be positive");
    }

    // Validate --post-only / --expires up front (no network calls needed).
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

    // Three resolution paths:
    //   1. --token-id fast path: skip all market lookup, get condition_id from book
    //   2. Series path: resolve series → GammaMarket once (avoids double Gamma fetch)
    //   3. Slug/condition_id standard path

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

    // Determine price (limit or market).
    let limit_price = if let Some(p) = price {
        if p <= 0.0 || p >= 1.0 {
            bail!("price must be in range (0, 1)");
        }
        let rp = round_price(p, tick_size);
        if rp <= 0.0 || rp >= 1.0 {
            bail!("price {p} rounds to {rp} with tick size {tick_size} — out of range (0, 1)");
        }
        rp
    } else if let Some(p) = compute_buy_worst_price(&book.asks, usdc_amount) {
        p
    } else {
        // No asks — convert market order to GTC limit at last trade price.
        let fallback = book.last_trade_price
            .as_deref()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|&p| p > 0.0 && p < 1.0)
            .map(|p| round_price(p, tick_size));
        let fp = fallback.ok_or_else(|| anyhow::anyhow!(
            "No asks in the order book and no last trade price available. \
             Pass --price to place a limit order manually."
        ))?;
        effective_order_type = "GTC";
        eprintln!(
            "[polymarket] No asks in order book — converting market order to GTC limit at \
             last trade price {:.4}. Pass --price to set a specific price.",
            fp
        );
        fp
    };

    // Build order amounts using integer arithmetic.
    //
    // Constraint: maker_amount_raw = price_ticks × taker_amount_raw / tick_scale
    // must be a non-negative integer (USDC in millionths).
    //
    // The minimum taker_amount_raw (shares in millionths) that satisfies this is:
    //   tick_scale / gcd(price_ticks, tick_scale)
    //
    // However, since Polymarket treats outcome token amounts as whole shares,
    // we align to 1 share (1_000_000 raw) as the minimum step. We then find
    // the smallest multiple of 1 share for which the USDC amount is also an integer.
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 { let t = b; b = a % b; a = t; }
        a
    }
    let tick_scale = (1.0 / tick_size).round() as u128;
    let price_ticks = (limit_price / tick_size).round() as u128;
    const SHARE_RAW: u128 = 1_000_000; // 1 whole share in raw units (6 decimal places)
    // Minimum k such that price_ticks × (k × SHARE_RAW) is divisible by tick_scale.
    // = tick_scale / gcd(price_ticks × SHARE_RAW, tick_scale)
    let g = gcd(price_ticks * SHARE_RAW % tick_scale.max(1), tick_scale.max(1));
    let shares_per_step = tick_scale.max(1) / g.max(1);
    // step is in share-raw units (millionths of shares)
    let step = shares_per_step * SHARE_RAW;

    let max_taker_raw = (usdc_amount / limit_price * 1_000_000.0).floor() as u128;
    let mut taker_amount_raw = if round_up {
        ((max_taker_raw + step - 1) / step) * step
    } else {
        (max_taker_raw / step) * step
    };
    let mut maker_amount_raw = price_ticks * taker_amount_raw / tick_scale;

    // Guard: amount too small.
    if taker_amount_raw == 0 || maker_amount_raw == 0 {
        let min_usdc = step as f64 / 1_000_000.0 * limit_price;
        bail!(
            "Amount too small: ${:.6} at price {:.4} rounds to 0 shares after divisibility \
             alignment. Minimum for this market/price is ~${:.6}. Pass --round-up to \
             automatically place the minimum amount instead.",
            usdc_amount, limit_price, min_usdc
        );
    }

    // Guard: resting orders below CLOB min_order_size are rejected.
    let min_order_size: f64 = book.min_order_size.as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let best_ask_float: Option<f64> = book.asks.last().and_then(|a| a.price.parse().ok());
    let is_resting = price.is_some() && best_ask_float.map_or(false, |ba| limit_price < ba);
    let computed_shares = taker_amount_raw as f64 / 1_000_000.0;
    if is_resting && min_order_size > 0.0 && computed_shares < min_order_size {
        if round_up {
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

    // ── Dry-run exit — full projected order fields ────────────────────────────
    if dry_run {
        use crate::config::Contracts;
        // Fetch CLOB version to show which exchange contract + collateral would be used.
        let dry_clob_version_raw = get_clob_version(&client).await?;
        let dry_clob_version = if dry_clob_version_raw == 2 { OrderVersion::V2 } else { OrderVersion::V1 };
        let dry_exchange_addr = Contracts::exchange(dry_clob_version, neg_risk);
        let dry_collateral = if dry_clob_version == OrderVersion::V2 { Contracts::PUSD } else { Contracts::USDC_E };
        let dry_version_label = if dry_clob_version == OrderVersion::V2 { "V2" } else { "V1" };

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
                    "side": "BUY",
                    "order_type": effective_order_type.to_uppercase(),
                    "limit_price": limit_price,
                    "usdc_amount": actual_usdc,
                    "usdc_requested": usdc_amount,
                    "shares": taker_amount_raw as f64 / 1_000_000.0,
                    "fee_rate_bps": fee_rate_bps,
                    "post_only": post_only,
                    "expires": if expiration > 0 { serde_json::Value::Number(expiration.into()) } else { serde_json::Value::Null },
                    "clob_version": dry_version_label,
                    "exchange_address": dry_exchange_addr,
                    "collateral_token": dry_collateral,
                    "neg_risk": neg_risk,
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

    // Resolve effective trading mode (one-time override > stored default).
    let effective_mode = match mode_override {
        Some("proxy") => TradingMode::PolyProxy,
        Some("eoa")   => TradingMode::Eoa,
        _             => creds.mode.clone(),
    };

    // Resolve maker address and signature type based on mode.
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

    let usdc_needed_raw = maker_amount_raw as u64;

    // Determine which address holds the USDC.e for this order.
    let balance_addr = match &effective_mode {
        TradingMode::PolyProxy => maker_addr.as_str(),
        TradingMode::Eoa       => signer_addr.as_str(),
    };

    // Fetch CLOB version and on-chain balances (USDC.e + pUSD) in parallel.
    // Version determines which collateral token and exchange contract to use:
    //   V1 → USDC.e + old exchange contracts
    //   V2 → pUSD  + new exchange contracts (V2 cutover ~2026-04-28)
    let (clob_version_raw, usdc_e_balance_result, pusd_balance_result) = tokio::join!(
        get_clob_version(&client),
        get_usdc_balance(balance_addr),
        get_pusd_balance(balance_addr),
    );
    let clob_version_raw = clob_version_raw?;
    let clob_version = if clob_version_raw == 2 { OrderVersion::V2 } else { OrderVersion::V1 };

    // Pre-flight balance check — collateral token depends on CLOB version.
    // V2 uses pUSD. If pUSD balance is insufficient but USDC.e balance is sufficient,
    // we automatically wrap USDC.e → pUSD via the Collateral Onramp before placing the order.
    match clob_version {
        OrderVersion::V2 => {
            let pusd_bal = pusd_balance_result.unwrap_or(0.0);
            let pusd_raw = (pusd_bal * 1_000_000.0).floor() as u64;

            // V2 server deducts order_amount + fee from pUSD at submission time.
            // Compute required pUSD including fee (ceiling division to avoid rounding short).
            let fee_buffer = ((usdc_needed_raw as u128 * fee_rate_bps as u128) + 9_999) / 10_000;
            let total_needed = usdc_needed_raw + fee_buffer as u64;

            // Pre-flight POL gas check (POLY_PROXY only):
            // V2 trading on a proxy may need up to two EOA-paid txs — USDC.e→pUSD wrap
            // and a one-time pUSD approve to V2 contracts. Surface the requirement up
            // front so the user isn't half-way through the flow when they run out of gas.
            // Skipped if no on-chain action is needed (pUSD already covers + already approved).
            if effective_mode == TradingMode::PolyProxy && !dry_run {
                let will_wrap = pusd_raw < total_needed;
                let exchange_addr = Contracts::exchange(clob_version, neg_risk);
                let pusd_allowance = crate::onchainos::get_pusd_allowance(
                    maker_addr.as_str(), exchange_addr,
                ).await.unwrap_or(0);
                let will_approve = pusd_allowance < (usdc_needed_raw as u128);
                if will_wrap || will_approve {
                    let pol = crate::onchainos::get_pol_balance(&signer_addr)
                        .await.unwrap_or(0.0);
                    const MIN_POL: f64 = 0.05;
                    if pol < MIN_POL {
                        let mut actions = Vec::new();
                        if will_wrap   { actions.push("USDC.e→pUSD wrap"); }
                        if will_approve { actions.push("V2 pUSD approve"); }
                        bail!(
                            "Insufficient POL gas on EOA wallet ({}) for V2 trading: have {:.4} POL, \
                             need ≥ {:.2} POL to cover the {} transaction(s). \
                             Send POL to your EOA on Polygon and retry.",
                            signer_addr, pol, MIN_POL, actions.join(" + ")
                        );
                    }
                }
            }

            if pusd_raw < total_needed {
                // pUSD insufficient — check USDC.e for auto-wrap opportunity.
                let usdc_e_bal = usdc_e_balance_result.unwrap_or(0.0);
                let usdc_e_raw = (usdc_e_bal * 1_000_000.0).floor() as u64;
                // Wrap only the shortfall: existing pUSD partially covers order + fee.
                let shortfall = total_needed - pusd_raw;
                if usdc_e_raw >= shortfall {
                    // Auto-wrap USDC.e → pUSD (shortfall only) before placing the order.
                    eprintln!(
                        "[polymarket] V2 requires pUSD collateral. pUSD balance ${:.6} < ${:.6} needed \
                         (order ${:.6} + fee ${:.6}). Auto-wrapping ${:.6} USDC.e → pUSD...",
                        pusd_bal,
                        total_needed as f64 / 1_000_000.0,
                        actual_usdc,
                        fee_buffer as f64 / 1_000_000.0,
                        shortfall as f64 / 1_000_000.0,
                    );
                    let wrap_tx = match &effective_mode {
                        TradingMode::Eoa => {
                            wrap_usdc_to_pusd(balance_addr, shortfall as u128).await?
                        }
                        TradingMode::PolyProxy => {
                            proxy_wrap_usdc_to_pusd(balance_addr, shortfall as u128).await?
                        }
                    };
                    eprintln!("[polymarket] Wrap tx: {}. Waiting for confirmation...", wrap_tx);
                    wait_for_tx_receipt(&wrap_tx, approve_timeout_secs()).await?;
                    eprintln!("[polymarket] Wrapped. Proceeding with order.");
                } else {
                    // Neither pUSD nor USDC.e is sufficient.
                    let total_needed_f64 = total_needed as f64 / 1_000_000.0;
                    let tip = match &effective_mode {
                        TradingMode::PolyProxy => format!(
                            "Run `polymarket deposit --amount {:.2}` to top up the proxy wallet, \
                             then the deposit will be auto-wrapped to pUSD on the next buy.",
                            total_needed_f64
                        ),
                        TradingMode::Eoa => {
                            let proxy_hint = crate::config::load_credentials()
                                .ok()
                                .flatten()
                                .and_then(|c| c.proxy_wallet)
                                .map(|proxy| format!(
                                    " Or switch to proxy mode (`polymarket switch-mode --mode proxy`) \
                                     if your USDC.e is in the proxy wallet ({}).",
                                    proxy
                                ))
                                .unwrap_or_default();
                            format!(
                                "Top up USDC.e on Polygon (it will be auto-wrapped to pUSD).{}",
                                proxy_hint
                            )
                        }
                    };
                    bail!(
                        "Insufficient balance for V2 order: have ${:.6} pUSD + ${:.6} USDC.e, \
                         need ${:.6} (order ${:.6} + fee ${:.6}). {}",
                        pusd_bal, usdc_e_bal,
                        total_needed_f64, actual_usdc,
                        fee_buffer as f64 / 1_000_000.0,
                        tip
                    );
                }
            }
        }
        OrderVersion::V1 => {
            // V1 uses USDC.e.
            match usdc_e_balance_result {
                Ok(bal_usdc) => {
                    let bal_raw = (bal_usdc * 1_000_000.0).floor() as u64;
                    if bal_raw < usdc_needed_raw {
                        let tip = match &effective_mode {
                            TradingMode::PolyProxy => format!(
                                "Run `polymarket deposit --amount {:.2}` to top up the proxy wallet.",
                                actual_usdc
                            ),
                            TradingMode::Eoa => {
                                let proxy_hint = crate::config::load_credentials()
                                    .ok()
                                    .flatten()
                                    .and_then(|c| c.proxy_wallet)
                                    .map(|proxy| format!(
                                        " Or switch to proxy mode (`polymarket switch-mode --mode proxy`) \
                                         if your USDC.e is already in the proxy wallet ({}).",
                                        proxy
                                    ))
                                    .unwrap_or_default();
                                format!(
                                    "Top up USDC.e on Polygon before placing this order.{}",
                                    proxy_hint
                                )
                            }
                        };
                        bail!(
                            "Insufficient USDC.e balance: have ${:.2}, need ${:.2}. {}",
                            bal_usdc, actual_usdc, tip
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[polymarket] Warning: could not verify on-chain USDC.e balance ({}); proceeding.", e);
                }
            }
        }
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

    // EOA mode: submit on-chain approve if allowance is insufficient.
    // POLY_PROXY mode: approvals are set once during `setup-proxy` — no per-trade approve needed.
    //
    // V2 migration: V2 uses a new exchange contract address. If the user has only approved V1,
    // the V2 allowance will be 0 and a fresh approval to the V2 contract is triggered automatically.
    if effective_mode == TradingMode::Eoa {
        let exchange_addr = Contracts::exchange(clob_version, neg_risk);
        // Use on-chain eth_call to read the live allowance — the CLOB API value can be
        // stale after an unlimited approval, causing a redundant approval on every order.
        let allowance_raw: u128 = match clob_version {
            OrderVersion::V2 => {
                let a = crate::onchainos::get_pusd_allowance(balance_addr, exchange_addr).await.unwrap_or(0);
                if neg_risk {
                    let b = crate::onchainos::get_pusd_allowance(balance_addr, Contracts::NEG_RISK_ADAPTER).await.unwrap_or(0);
                    a.min(b)
                } else { a }
            }
            OrderVersion::V1 => {
                let a = crate::onchainos::get_usdc_allowance(balance_addr, exchange_addr).await.unwrap_or(0);
                if neg_risk {
                    let b = crate::onchainos::get_usdc_allowance(balance_addr, Contracts::NEG_RISK_ADAPTER).await.unwrap_or(0);
                    a.min(b)
                } else { a }
            }
        };

        if allowance_raw < usdc_needed_raw as u128 || auto_approve {
            let (version_label, collateral_label) = if clob_version == OrderVersion::V2 {
                (" V2", "pUSD")
            } else {
                ("", "USDC.e")
            };
            let exchange_label = if neg_risk {
                format!("Neg Risk CTF Exchange{}", version_label)
            } else {
                format!("CTF Exchange{}", version_label)
            };
            eprintln!("[polymarket] Approving unlimited {} for {} (one-time)...", collateral_label, exchange_label);
            let tx_hash = approve_usdc_versioned(neg_risk, clob_version, usdc_needed_raw).await?;
            eprintln!("[polymarket] Approval tx: {}", tx_hash);
            eprintln!("[polymarket] Waiting for approval to confirm on-chain...");
            crate::onchainos::wait_for_tx_receipt(&tx_hash, approve_timeout_secs()).await?;
            eprintln!("[polymarket] Approval confirmed.");
        }
    } else if effective_mode == TradingMode::PolyProxy && clob_version == OrderVersion::V2 {
        // POLY_PROXY + V2: query pUSD allowance on-chain (not via CLOB API).
        // The CLOB /balance-allowance endpoint hard-codes signature_type=0, which scopes
        // the lookup to the EOA address — but the V2 pUSD approvals live on the proxy
        // wallet (set by setup-proxy or a prior lazy approve). Querying on-chain matches
        // the source of truth and prevents redundant approve txs on every buy.
        // Idempotent — unlimited approval (maxUint) means it only fires once.
        let exchange_addr = Contracts::exchange(clob_version, neg_risk);
        let needed_u128 = usdc_needed_raw as u128;
        let allowance_raw = if neg_risk {
            let a_exchange = crate::onchainos::get_pusd_allowance(maker_addr.as_str(), exchange_addr)
                .await.unwrap_or(0);
            let a_adapter  = crate::onchainos::get_pusd_allowance(maker_addr.as_str(), Contracts::NEG_RISK_ADAPTER)
                .await.unwrap_or(0);
            a_exchange.min(a_adapter)
        } else {
            crate::onchainos::get_pusd_allowance(maker_addr.as_str(), exchange_addr)
                .await.unwrap_or(0)
        };
        if allowance_raw < needed_u128 {
            let version_label = if neg_risk { "Neg Risk CTF Exchange V2" } else { "CTF Exchange V2" };
            eprintln!("[polymarket] Proxy pUSD allowance insufficient for {}. Approving via proxy...", version_label);
            let tx = proxy_pusd_approve(exchange_addr).await?;
            eprintln!("[polymarket] Proxy pUSD approval tx: {}. Waiting for confirmation...", tx);
            wait_for_tx_receipt(&tx, approve_timeout_secs()).await?;
            if neg_risk {
                eprintln!("[polymarket] Approving pUSD for Neg Risk Adapter via proxy...");
                let tx2 = proxy_pusd_approve(Contracts::NEG_RISK_ADAPTER).await?;
                wait_for_tx_receipt(&tx2, approve_timeout_secs()).await?;
            }
            eprintln!("[polymarket] Proxy pUSD approval confirmed.");
        }
    }
    // POLY_PROXY V1: approvals (USDC.e) are set once at setup-proxy time — no per-trade check needed.

    let salt = rand_salt();

    // Sign and submit the order using the correct version's struct and exchange contract.
    let resp = match clob_version {
        OrderVersion::V2 => {
            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let params = OrderParamsV2 {
                salt,
                maker: maker_addr.clone(),
                signer: signer_addr.clone(),
                token_id: token_id.clone(),
                maker_amount: maker_amount_raw as u64,
                taker_amount: taker_amount_raw as u64,
                side: 0, // BUY
                signature_type: sig_type,
                timestamp_ms,
                metadata: BYTES32_ZERO.to_string(),
                builder: BYTES32_ZERO.to_string(),
            };
            let signature = sign_order_v2_via_onchainos(&params, neg_risk).await?;
            let order_body = OrderBodyV2 {
                salt,
                maker: maker_addr.clone(),
                signer: signer_addr.clone(),
                token_id: token_id.clone(),
                maker_amount: maker_amount_raw.to_string(),
                taker_amount: taker_amount_raw.to_string(),
                side: "BUY".to_string(),
                signature_type: sig_type,
                timestamp: timestamp_ms.to_string(),
                metadata: BYTES32_ZERO.to_string(),
                builder: BYTES32_ZERO.to_string(),
                signature,
            };
            // In V2, expiration moves to the outer wrapper (not part of the signed struct).
            let order_req = OrderRequestV2 {
                order: order_body,
                owner: creds.api_key.clone(),
                order_type: effective_order_type.to_uppercase(),
                post_only,
                expiration: if expiration > 0 { expiration.to_string() } else { String::new() },
            };
            post_order(&client, &signer_addr, &creds, &order_req).await?
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
                side: 0, // BUY
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
                side: "BUY".to_string(),
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

    let actual_shares = taker_amount_raw as f64 / 1_000_000.0;
    // Attribution: report every buy that produced an order_id.
    // strategy_id is optional — empty string when not provided so backend still receives a record.
    if let Some(oid) = resp.order_id.as_deref().filter(|s| !s.is_empty()) {
        let ts_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let sid = strategy_id.unwrap_or("");
        let report_payload = serde_json::json!({
            "wallet": signer_addr,
            "proxyAddress": creds.proxy_wallet.as_deref().unwrap_or(""),
            "order_id": oid,
            "tx_hashes": resp.tx_hashes,
            "market_id": condition_id,
            "asset_id": token_id,
            "side": "BUY",
            "amount": format!("{}", actual_shares),
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
            "shares": actual_shares,
            "rounded_up": round_up && actual_usdc > usdc_amount + 1e-6,
            "post_only": post_only,
            "expires": if expiration > 0 { serde_json::Value::Number(expiration.into()) } else { serde_json::Value::Null },
            "tx_hashes": resp.tx_hashes,
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

/// Resolve (condition_id, token_id, neg_risk, fee_rate_bps) from a market_id and outcome string.
/// Supports any outcome label (e.g. "yes", "no", "trump", "republican", "option-a").
/// Bails early if the market is not accepting orders (closed, resolved, or paused).
///
/// neg_risk and fee_rate_bps are always sourced from the CLOB API (authoritative) because the
/// Gamma API omits the negRisk field for many markets, causing incorrect contract approval targets.
pub async fn resolve_market_token(
    client: &Client,
    market_id: &str,
    outcome: &str,
) -> Result<(String, String, bool, u64)> {
    let outcome_lower = outcome.to_lowercase();
    if market_id.starts_with("0x") || market_id.starts_with("0X") {
        let market = get_clob_market(client, market_id).await?;
        if !market.accepting_orders {
            bail!(
                "Market {} is not accepting orders (closed or resolved). \
                 Use `polymarket get-market` to check its current status.",
                market_id
            );
        }
        let token = market
            .tokens
            .iter()
            .find(|t| t.outcome.to_lowercase() == outcome_lower)
            .ok_or_else(|| {
                let available: Vec<&str> = market.tokens.iter().map(|t| t.outcome.as_str()).collect();
                anyhow::anyhow!("Outcome '{}' not found. Available outcomes: {:?}", outcome, available)
            })?;
        let fee = market.maker_base_fee.unwrap_or(0);
        Ok((market.condition_id.clone(), token.token_id.clone(), market.neg_risk, fee))
    } else {
        let gamma = crate::api::get_gamma_market_by_slug(client, market_id).await?;
        if !gamma.accepting_orders {
            bail!(
                "Market '{}' is not accepting orders (closed or resolved). \
                 Use `polymarket get-market` to check its current status.",
                market_id
            );
        }
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

        // Get authoritative neg_risk and fee from CLOB — Gamma API omits negRisk for many
        // markets, which causes the wrong exchange to be approved (CTF_EXCHANGE instead of
        // NEG_RISK_CTF_EXCHANGE), wasting gas and failing the order.
        let (neg_risk, fee) = match get_clob_market(client, &condition_id).await {
            Ok(clob) => (clob.neg_risk, clob.maker_base_fee.unwrap_or(0)),
            Err(_) => (gamma.neg_risk, 0), // fall back to gamma value if CLOB unavailable
        };

        Ok((condition_id, token_id, neg_risk, fee))
    }
}

/// Resolve (condition_id, token_id, neg_risk, fee_rate_bps) from a pre-fetched GammaMarket.
/// Used in the series path to avoid fetching the same Gamma market twice.
pub async fn resolve_from_gamma(
    client: &Client,
    gamma: crate::api::GammaMarket,
    outcome: &str,
) -> Result<(String, String, bool, u64)> {
    if !gamma.accepting_orders {
        bail!("Series market is not currently accepting orders. It may be outside trading hours or in a transition window.");
    }
    let outcome_lower = outcome.to_lowercase();
    let condition_id = gamma.condition_id.clone()
        .ok_or_else(|| anyhow::anyhow!("No condition_id in Gamma market response"))?;
    let token_ids = gamma.token_ids();
    let outcomes = gamma.outcome_list();
    let idx = outcomes.iter().position(|o| o.to_lowercase() == outcome_lower)
        .ok_or_else(|| anyhow::anyhow!("Outcome '{}' not found. Available outcomes: {:?}", outcome, outcomes))?;
    let token_id = token_ids.get(idx).cloned()
        .ok_or_else(|| anyhow::anyhow!("No token_id for outcome index {}", idx))?;
    let (neg_risk, fee_rate_bps) = match get_clob_market(client, &condition_id).await {
        Ok(clob) => (clob.neg_risk, clob.maker_base_fee.unwrap_or(0)),
        Err(_) => (gamma.neg_risk, 0),
    };
    Ok((condition_id, token_id, neg_risk, fee_rate_bps))
}

/// Generate a random salt within JavaScript's safe integer range (< 2^53).
fn rand_salt() -> u64 {
    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).expect("getrandom failed");
    u64::from_le_bytes(bytes) & 0x001F_FFFF_FFFF_FFFF
}

/// Approve the collateral token for the correct exchange contract based on CLOB version.
///
/// V1 → approves USDC.e to CTF_EXCHANGE (or NEG_RISK_CTF_EXCHANGE for neg-risk).
/// V2 → approves pUSD to CTF_EXCHANGE_V2 (or NEG_RISK_CTF_EXCHANGE_V2 for neg-risk).
///
/// pUSD (Polymarket USD) replaced USDC.e as collateral for V2 exchange contracts
/// from ~2026-04-28. This function routes to the correct token automatically so users
/// get a V2 pUSD approval on their first V2 trade without any manual intervention.
async fn approve_usdc_versioned(
    neg_risk: bool,
    version: OrderVersion,
    _amount_raw: u64,
) -> anyhow::Result<String> {
    use crate::config::Contracts;
    use crate::onchainos::usdc_approve;

    let collateral_token = match version {
        OrderVersion::V2 => Contracts::PUSD,
        OrderVersion::V1 => Contracts::USDC_E,
    };
    let exchange_addr = Contracts::exchange(version, neg_risk);

    // Always approve u128::MAX — setting an exact amount causes unnecessary re-approval
    // on every trade when a MAX_UINT allowance was already granted previously.
    if neg_risk {
        let adapter_addr = Contracts::NEG_RISK_ADAPTER;
        usdc_approve(collateral_token, exchange_addr, u128::MAX).await?;
        return usdc_approve(collateral_token, adapter_addr, u128::MAX).await;
    }

    usdc_approve(collateral_token, exchange_addr, u128::MAX).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // Serialize env-var tests so they don't contaminate each other when run in parallel.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // ── Bug #6: Approval timeout env var ────────────────────────────────────

    /// Default timeout is 90 seconds when env var is not set.
    /// Rationale: Polygon block time ~2s; 30s was too short for congested periods.
    #[test]
    fn test_approve_timeout_default() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("POLYMARKET_APPROVE_TIMEOUT_SECS");
        assert_eq!(approve_timeout_secs(), 90, "default timeout should be 90s");
    }

    /// Env var override is respected and parsed correctly.
    #[test]
    fn test_approve_timeout_env_override() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("POLYMARKET_APPROVE_TIMEOUT_SECS", "120");
        let t = approve_timeout_secs();
        std::env::remove_var("POLYMARKET_APPROVE_TIMEOUT_SECS");
        assert_eq!(t, 120, "env var should override default");
    }

    /// Invalid env var value falls back to default (no panic).
    #[test]
    fn test_approve_timeout_invalid_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        std::env::set_var("POLYMARKET_APPROVE_TIMEOUT_SECS", "not_a_number");
        let t = approve_timeout_secs();
        std::env::remove_var("POLYMARKET_APPROVE_TIMEOUT_SECS");
        assert_eq!(t, 90, "invalid env var value should fall back to default 90s");
    }
}
