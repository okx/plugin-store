use clap::Args;
use crate::api::{get_asset_meta, get_all_mids, get_clearinghouse_state};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_standalone_tpsl_action, format_px, round_px, submit_exchange_request};

#[derive(Args)]
pub struct TpslArgs {
    /// Coin (e.g. BTC, ETH, SOL)
    #[arg(long)]
    pub coin: String,

    /// Stop-loss trigger price — order fires when mark price crosses this level
    #[arg(long)]
    pub sl_px: Option<f64>,

    /// Take-profit trigger price — order fires when mark price crosses this level
    #[arg(long)]
    pub tp_px: Option<f64>,

    /// Override position size (auto-detected from current position by default)
    #[arg(long)]
    pub size: Option<String>,

    /// Dry run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: TpslArgs) -> anyhow::Result<()> {
    if args.sl_px.is_none() && args.tp_px.is_none() {
        anyhow::bail!("Provide at least one of --sl-px (stop-loss) or --tp-px (take-profit)");
    }

    let info = info_url();
    let exchange = exchange_url();
    let coin = normalize_coin(&args.coin);
    let nonce = now_ms();

    let (asset_idx, sz_decimals) = get_asset_meta(info, &coin).await?;
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Auto-detect position direction and size
    let state = get_clearinghouse_state(info, &wallet).await?;
    let empty_vec = vec![];
    let positions = state["assetPositions"].as_array().unwrap_or(&empty_vec);

    let mut position_szi: Option<f64> = None;
    let mut entry_px_str = String::from("unknown");
    let mut liquidation_px_str = String::from("none");

    for pw in positions {
        let pos = &pw["position"];
        if pos["coin"].as_str().map(|c| c.to_uppercase()) == Some(coin.to_uppercase()) {
            if let Some(s) = pos["szi"].as_str() {
                position_szi = s.parse().ok();
                entry_px_str = pos["entryPx"].as_str().unwrap_or("?").to_string();
                liquidation_px_str = pos["liquidationPx"]
                    .as_str()
                    .unwrap_or("none")
                    .to_string();
                break;
            }
        }
    }

    let szi = position_szi.ok_or_else(|| {
        anyhow::anyhow!(
            "No open {} position found. Use `hyperliquid positions` to check.",
            coin
        )
    })?;

    if szi == 0.0 {
        anyhow::bail!("No open {} position (size is 0).", coin);
    }

    let position_is_long = szi > 0.0;
    let position_size = szi.abs();
    let position_side = if position_is_long { "long" } else { "short" };

    // Validate TP/SL prices make sense relative to position direction
    let mids = get_all_mids(info).await?;
    let current_price_str = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let current_price: f64 = current_price_str.parse().unwrap_or(0.0);

    if let Some(sl) = args.sl_px {
        if position_is_long && sl >= current_price {
            anyhow::bail!(
                "Stop-loss {} is above current price {} for a long position. \
                 SL must be below current price.",
                sl, current_price_str
            );
        }
        if !position_is_long && sl <= current_price {
            anyhow::bail!(
                "Stop-loss {} is below current price {} for a short position. \
                 SL must be above current price.",
                sl, current_price_str
            );
        }
    }
    if let Some(tp) = args.tp_px {
        if position_is_long && tp <= current_price {
            anyhow::bail!(
                "Take-profit {} is below current price {} for a long position. \
                 TP must be above current price.",
                tp, current_price_str
            );
        }
        if !position_is_long && tp >= current_price {
            anyhow::bail!(
                "Take-profit {} is above current price {} for a short position. \
                 TP must be below current price.",
                tp, current_price_str
            );
        }
    }

    // Determine order size
    let size_str = match &args.size {
        Some(s) => {
            let v: f64 = s
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid --size '{}'", s))?;
            if v <= 0.0 || v > position_size {
                anyhow::bail!("--size must be > 0 and ≤ position size {}", position_size);
            }
            s.clone()
        }
        None => format!("{}", position_size),
    };

    // Round TP/SL prices to the correct precision for this coin (matches HL's szDecimals rule).
    // format_px uses raw 6-decimal truncation; round_px applies significant-figure rounding
    // so BTC prices become integers, ETH/SOL prices round to the correct decimal count.
    let sl_px_str = args.sl_px.map(|px| round_px(px, sz_decimals));
    let tp_px_str = args.tp_px.map(|px| round_px(px, sz_decimals));

    let action = build_standalone_tpsl_action(
        asset_idx,
        position_is_long,
        &size_str,
        sl_px_str.as_deref(),
        tp_px_str.as_deref(),
        sz_decimals,
    );

    // Compute implied slippage limit prices for display
    let sl_limit_display = args.sl_px.map(|px| {
        let closing_is_buy = !position_is_long;
        let limit = if closing_is_buy { px * 1.1 } else { px * 0.9 };
        round_px(limit, sz_decimals)
    });
    let tp_limit_display = args.tp_px.map(|px| {
        let closing_is_buy = !position_is_long;
        let limit = if closing_is_buy { px * 1.1 } else { px * 0.9 };
        round_px(limit, sz_decimals)
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "coin": coin,
                "positionSide": position_side,
                "positionSize": position_size,
                "orderSize": size_str,
                "entryPrice": entry_px_str,
                "currentMidPrice": current_price_str,
                "liquidationPrice": liquidation_px_str,
                "stopLoss": args.sl_px.map(|px| serde_json::json!({
                    "triggerPx": round_px(px, sz_decimals),
                    "executionType": "market",
                    "worstFillPx": sl_limit_display
                })),
                "takeProfit": args.tp_px.map(|px| serde_json::json!({
                    "triggerPx": round_px(px, sz_decimals),
                    "executionType": "market",
                    "worstFillPx": tp_limit_display
                })),
                "nonce": nonce
            },
            "action": action
        }))?
    );

    if args.dry_run {
        eprintln!("\n[DRY RUN] Not signed or submitted.");
        return Ok(());
    }

    if !args.confirm {
        eprintln!("\n[PREVIEW] Add --confirm to place these TP/SL orders.");
        eprintln!("NOTE: Both orders are sent independently (grouping: na). \
                  The first one to trigger closes your position; cancel the \
                  other manually afterward.");
        return Ok(());
    }

    let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "action": "tpsl",
            "coin": coin,
            "positionSide": position_side,
            "stopLoss": sl_px_str,
            "takeProfit": tp_px_str,
            "result": result
        }))?
    );

    Ok(())
}
