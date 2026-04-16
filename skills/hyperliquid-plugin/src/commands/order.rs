use clap::Args;
use crate::api::{get_asset_meta, get_all_mids, get_clearinghouse_state, get_spot_clearinghouse_state};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID, USDC_ARBITRUM};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::rpc::{ARBITRUM_RPC, erc20_balance};
use crate::signing::{
    build_bracketed_order_action, build_limit_order_action, build_market_order_action,
    build_update_leverage_action,
    format_px, round_px, market_slippage_px, submit_exchange_request,
};

#[derive(Args)]
pub struct OrderArgs {
    /// Coin to trade (e.g. BTC, ETH, SOL, ARB)
    #[arg(long)]
    pub coin: String,

    /// Side: buy (long) or sell (short)
    #[arg(long, value_parser = ["buy", "sell"])]
    pub side: String,

    /// Position size in base units (e.g. 0.01 for 0.01 BTC)
    #[arg(long)]
    pub size: String,

    /// Order type: market or limit
    #[arg(long, value_parser = ["market", "limit"], default_value = "market")]
    pub r#type: String,

    /// Limit price (required for limit orders)
    #[arg(long)]
    pub price: Option<String>,

    /// Stop-loss trigger price — attaches a stop-loss child order (bracket)
    #[arg(long)]
    pub sl_px: Option<f64>,

    /// Take-profit trigger price — attaches a take-profit child order (bracket)
    #[arg(long)]
    pub tp_px: Option<f64>,

    /// Leverage multiplier before placing (e.g. 10 for 10x cross). Sets account leverage for this
    /// coin first, then places the order. Omit to keep the current account setting.
    #[arg(long)]
    pub leverage: Option<u32>,

    /// Use isolated margin mode when --leverage is set (default is cross margin)
    #[arg(long)]
    pub isolated: bool,

    /// Reduce only — only reduce an existing position, never increase it
    #[arg(long)]
    pub reduce_only: bool,

    /// Dry run — preview order payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit the order (without this flag, prints a preview)
    #[arg(long)]
    pub confirm: bool,
}

/// Format a size value to exactly `decimals` decimal places, trimming trailing zeros.
fn fmt_size(sz: f64, decimals: u32) -> String {
    if decimals == 0 {
        format!("{:.0}", sz)
    } else {
        let s = format!("{:.prec$}", sz, prec = decimals as usize);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

pub async fn run(args: OrderArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();
    let coin = normalize_coin(&args.coin);
    let is_buy = args.side.to_lowercase() == "buy";
    let nonce = now_ms();

    // Validate size is a number
    let size_f: f64 = args
        .size
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid size '{}' — must be a number (e.g. 0.01)", args.size))?;

    // Validate leverage range (Hyperliquid accepts 1–100)
    if let Some(lev) = args.leverage {
        if !(1..=100).contains(&lev) {
            anyhow::bail!("--leverage must be between 1 and 100 (got {})", lev);
        }
    }

    // TP/SL bracket validation
    if let Some(sl) = args.sl_px {
        if is_buy && args.tp_px.map_or(false, |tp| tp <= sl) {
            anyhow::bail!("Take-profit must be above stop-loss for a long position");
        }
        if !is_buy && args.tp_px.map_or(false, |tp| tp >= sl) {
            anyhow::bail!("Take-profit must be below stop-loss for a short position");
        }
    }

    // ─── Fetch meta + prices concurrently ────────────────────────────────────
    let ((asset_idx, sz_decimals), mids) = tokio::try_join!(
        get_asset_meta(info, &coin),
        get_all_mids(info)
    )?;

    let current_price = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let mid_f = current_price.parse::<f64>().unwrap_or(0.0);

    // ─── Size: round to szDecimals, then auto-bump if notional < $10 ─────────
    let sz_factor = 10_f64.powi(sz_decimals as i32);
    let mut size_rounded = (size_f * sz_factor).round() / sz_factor;

    if mid_f > 0.0 {
        let n = size_rounded * mid_f;
        if n > 0.0 && n < 10.0 {
            let bumped = size_rounded + 1.0 / sz_factor;
            eprintln!(
                "[auto-adjust] size {} → {} to meet $10 minimum notional (${:.2} → ${:.2})",
                fmt_size(size_rounded, sz_decimals),
                fmt_size(bumped, sz_decimals),
                n,
                bumped * mid_f,
            );
            size_rounded = bumped;
        }
    }
    let size_str = fmt_size(size_rounded, sz_decimals);
    let notional = size_rounded * mid_f;

    // Slippage-protected price for market orders
    let slippage_px_str = market_slippage_px(mid_f, is_buy, sz_decimals);

    // ─── SL/TP prices rounded to correct precision ────────────────────────────
    let sl_px_str = args.sl_px.map(|px| round_px(px, sz_decimals));
    let tp_px_str = args.tp_px.map(|px| round_px(px, sz_decimals));

    // ─── Balance pre-flight (non-fatal — skip if wallet not connected) ────────
    // Shows Perp + Spot + Arbitrum. HyperEVM excluded per user preference.
    let wallet_opt = resolve_wallet(CHAIN_ID).ok();
    let arb_wallet_opt = resolve_wallet(ARBITRUM_CHAIN_ID).ok();

    struct Balances {
        perp: f64,
        spot: f64,
        arb: f64,
    }

    let balances_opt: Option<Balances> = if let Some(ref w) = wallet_opt {
        let aw_clone = arb_wallet_opt.clone();
        let (perp_res, spot_res, arb_raw) = tokio::join!(
            get_clearinghouse_state(info, w),
            get_spot_clearinghouse_state(info, w),
            async move {
                match aw_clone.as_deref() {
                    Some(aw) => erc20_balance(USDC_ARBITRUM, aw, ARBITRUM_RPC)
                        .await
                        .unwrap_or(0),
                    None => 0u128,
                }
            }
        );

        let perp = perp_res
            .ok()
            .and_then(|s| s["withdrawable"].as_str()?.parse::<f64>().ok())
            .unwrap_or(0.0);

        let spot = spot_res
            .ok()
            .and_then(|s| {
                s["balances"].as_array()?.iter()
                    .find(|b| b["coin"].as_str() == Some("USDC"))?
                    ["total"]
                    .as_str()?
                    .parse::<f64>()
                    .ok()
            })
            .unwrap_or(0.0);

        Some(Balances { perp, spot, arb: arb_raw as f64 / 1_000_000.0 })
    } else {
        None
    };

    // Estimate required margin; default to 10x if --leverage not provided
    let effective_leverage = args.leverage.map(|l| l as f64).unwrap_or(10.0);
    let required_margin = if notional > 0.0 { notional / effective_leverage } else { 0.0 };

    // Build balance landscape JSON (included in preview + stop output)
    let balance_json = balances_opt.as_ref().map(|b| {
        serde_json::json!({
            "perp_withdrawable": format!("{:.4}", b.perp),
            "spot_usdc":         format!("{:.4}", b.spot),
            "arbitrum_usdc":     format!("{:.4}", b.arb),
            "total_usdc":        format!("{:.4}", b.perp + b.spot + b.arb),
        })
    });

    // Gate: STOP if perp balance is clearly insufficient
    if let Some(ref b) = balances_opt {
        if b.perp < required_margin {
            let shortfall = required_margin - b.perp;
            let tip = if b.spot >= shortfall {
                format!(
                    "Spot has enough USDC. Run: hyperliquid transfer --amount {:.2} --from spot",
                    shortfall
                )
            } else if b.arb >= shortfall {
                format!(
                    "Arbitrum has enough USDC. Run: hyperliquid deposit --amount {:.2}",
                    shortfall
                )
            } else {
                format!(
                    "Total across all accounts: ${:.2}. Add ${:.2} more USDC (e.g. via `hyperliquid deposit`).",
                    b.perp + b.spot + b.arb,
                    shortfall
                )
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": "Insufficient perp balance",
                    "notional_usd": format!("${:.2}", notional),
                    "estimated_leverage": format!("{}x", effective_leverage as u32),
                    "required_margin_est": format!("${:.4}", required_margin),
                    "shortfall": format!("${:.4}", shortfall),
                    "fund_landscape": balance_json,
                    "tip": tip,
                }))?
            );
            return Ok(());
        }
    }

    // ─── Build action ────────────────────────────────────────────────────────
    let has_bracket = args.sl_px.is_some() || args.tp_px.is_some();

    let action = if has_bracket {
        let entry_element = match args.r#type.as_str() {
            "market" => serde_json::json!({
                "a": asset_idx,
                "b": is_buy,
                "p": slippage_px_str,
                "s": size_str,
                "r": args.reduce_only,
                "t": { "limit": { "tif": "Ioc" } }
            }),
            "limit" => {
                let price_str = args
                    .price
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("--price is required for limit orders"))?;
                let _: f64 = price_str
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid price '{}'", price_str))?;
                serde_json::json!({
                    "a": asset_idx,
                    "b": is_buy,
                    "p": price_str,
                    "s": size_str,
                    "r": args.reduce_only,
                    "t": { "limit": { "tif": "Gtc" } }
                })
            }
            _ => anyhow::bail!("Unknown order type '{}'", args.r#type),
        };

        build_bracketed_order_action(
            entry_element,
            asset_idx,
            is_buy,
            &size_str,
            sl_px_str.as_deref(),
            tp_px_str.as_deref(),
            sz_decimals,
        )
    } else {
        match args.r#type.as_str() {
            "market" => build_market_order_action(asset_idx, is_buy, &size_str, args.reduce_only, &slippage_px_str),
            "limit" => {
                let price_str = args
                    .price
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("--price is required for limit orders"))?;
                let _: f64 = price_str
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid price '{}'", price_str))?;
                build_limit_order_action(asset_idx, is_buy, price_str, &size_str, args.reduce_only, "Gtc")
            }
            _ => anyhow::bail!("Unknown order type '{}'", args.r#type),
        }
    };

    let leverage_preview = args.leverage.map(|l| {
        format!("{}x {}", l, if args.isolated { "isolated" } else { "cross" })
    });

    // ─── Preview ─────────────────────────────────────────────────────────────
    let mut preview_obj = serde_json::json!({
        "coin": coin,
        "assetIndex": asset_idx,
        "side": args.side,
        "size": size_str,
        "notional_usd": format!("{:.2}", notional),
        "type": args.r#type,
        "price": args.price,
        "leverage": leverage_preview,
        "stopLoss": sl_px_str,
        "takeProfit": tp_px_str,
        "reduceOnly": args.reduce_only,
        "currentMidPrice": current_price,
        "grouping": if has_bracket { "normalTpsl" } else { "na" },
        "nonce": nonce
    });
    if let Some(ref bj) = balance_json {
        preview_obj["fund_landscape"] = bj.clone();
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": preview_obj,
            "action": action
        }))?
    );

    if args.dry_run {
        eprintln!("\n[DRY RUN] Order not signed or submitted.");
        return Ok(());
    }

    if !args.confirm {
        eprintln!("\n[PREVIEW] Add --confirm to sign and submit this order.");
        eprintln!("WARNING: This will place a real perpetual order on Hyperliquid.");
        eprintln!("         Perpetuals trading involves significant risk including total loss.");
        return Ok(());
    }

    // ─── Submit ───────────────────────────────────────────────────────────────
    let wallet = wallet_opt
        .ok_or_else(|| anyhow::anyhow!("Cannot resolve wallet. Log in via onchainos."))?;

    // Set leverage before placing the order if --leverage was provided
    if let Some(lev) = args.leverage {
        let is_cross = !args.isolated;
        let lev_action = build_update_leverage_action(asset_idx, is_cross, lev);
        let lev_nonce = now_ms();
        let lev_signed = onchainos_hl_sign(&lev_action, lev_nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
        let lev_result = submit_exchange_request(exchange, lev_signed).await
            .map_err(|e| anyhow::anyhow!("Leverage update failed: {}", e))?;
        if lev_result["status"].as_str() == Some("err") {
            anyhow::bail!(
                "Leverage update rejected by Hyperliquid: {}",
                lev_result["response"].as_str().unwrap_or("unknown error")
            );
        }
        eprintln!(
            "Leverage set to {}x ({}) for {}",
            lev, if is_cross { "cross" } else { "isolated" }, coin
        );
    }

    let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "coin": coin,
            "side": args.side,
            "size": size_str,
            "notional_usd": format!("{:.2}", notional),
            "type": args.r#type,
            "stopLoss": sl_px_str,
            "takeProfit": tp_px_str,
            "result": result
        }))?
    );

    Ok(())
}
