use clap::Args;
use crate::api::{
    fetch_perp_dexs, get_all_mids, get_asset_meta_for_coin, get_clearinghouse_state_for_dex,
    parse_coin,
};
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

    /// Worst-fill slippage tolerance when the trigger fires, in percent (default 10.0 = 10%).
    /// The limit price is set to trigger_px × (1 ± trigger_slippage/100).
    #[arg(long, default_value = "10.0")]
    pub trigger_slippage: f64,

    /// Dry run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: TpslArgs) -> anyhow::Result<()> {
    if args.sl_px.is_none() && args.tp_px.is_none() {
        println!("{}", super::error_response(
            "Provide at least one of --sl-px (stop-loss) or --tp-px (take-profit)",
            "INVALID_ARGUMENT",
            "Example: hyperliquid tpsl --coin BTC --sl-px 90000 --tp-px 110000"
        ));
        return Ok(());
    }

    if args.trigger_slippage <= 0.0 || args.trigger_slippage > 100.0 {
        println!("{}", super::error_response(
            &format!("--trigger-slippage must be between 0 and 100 (got {})", args.trigger_slippage),
            "INVALID_ARGUMENT",
            "Provide a trigger slippage value between 0 and 100, e.g. --trigger-slippage 10.0"
        ));
        return Ok(());
    }

    let info = info_url();
    let exchange = exchange_url();
    // HIP-3: parse dex prefix
    let (dex_opt, _) = parse_coin(&args.coin);
    let coin = if dex_opt.is_some() {
        let (d, b) = parse_coin(&args.coin);
        format!("{}:{}", d.unwrap(), b.to_uppercase())
    } else {
        normalize_coin(&args.coin)
    };
    let nonce = now_ms();

    let registry = fetch_perp_dexs(info).await.unwrap_or_default();
    let (asset_idx, sz_decimals) = match get_asset_meta_for_coin(info, &coin, &registry).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry. If using a builder DEX coin (e.g. xyz:CL), run `hyperliquid-plugin dex-list`."));
            return Ok(());
        }
    };
    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };

    // Auto-detect position direction and size (dex-aware)
    let state = match get_clearinghouse_state_for_dex(info, &wallet, dex_opt.as_deref()).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
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

    let szi = match position_szi {
        Some(v) => v,
        None => {
            println!("{}", super::error_response(
                &format!("No open {} position found.", coin),
                "POSITION_NOT_FOUND",
                "Run positions to see open positions."
            ));
            return Ok(());
        }
    };

    if szi == 0.0 {
        println!("{}", super::error_response(
            &format!("No open {} position (size is 0).", coin),
            "POSITION_NOT_FOUND",
            "Run positions to see open positions."
        ));
        return Ok(());
    }

    let position_is_long = szi > 0.0;
    let position_size = szi.abs();
    let position_side = if position_is_long { "long" } else { "short" };

    // Validate TP/SL prices make sense relative to position direction
    let mids = match get_all_mids(info).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
    let current_price_str = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    let current_price: f64 = current_price_str.parse().unwrap_or(0.0);

    if let Some(sl) = args.sl_px {
        if position_is_long && sl >= current_price {
            println!("{}", super::error_response(
                &format!("Stop-loss {} is above current price {} for a long position. SL must be below current price.", sl, current_price_str),
                "INVALID_ARGUMENT",
                "For a long position, stop-loss must be below the current price."
            ));
            return Ok(());
        }
        if !position_is_long && sl <= current_price {
            println!("{}", super::error_response(
                &format!("Stop-loss {} is below current price {} for a short position. SL must be above current price.", sl, current_price_str),
                "INVALID_ARGUMENT",
                "For a short position, stop-loss must be above the current price."
            ));
            return Ok(());
        }
    }
    if let Some(tp) = args.tp_px {
        if position_is_long && tp <= current_price {
            println!("{}", super::error_response(
                &format!("Take-profit {} is below current price {} for a long position. TP must be above current price.", tp, current_price_str),
                "INVALID_ARGUMENT",
                "For a long position, take-profit must be above the current price."
            ));
            return Ok(());
        }
        if !position_is_long && tp >= current_price {
            println!("{}", super::error_response(
                &format!("Take-profit {} is above current price {} for a short position. TP must be below current price.", tp, current_price_str),
                "INVALID_ARGUMENT",
                "For a short position, take-profit must be below the current price."
            ));
            return Ok(());
        }
    }

    // Determine order size
    let size_str = match &args.size {
        Some(s) => {
            let v: f64 = match s.parse() {
                Ok(v) => v,
                Err(_) => {
                    println!("{}", super::error_response(
                        &format!("Invalid --size '{}'", s),
                        "INVALID_ARGUMENT",
                        "Provide a numeric size value, e.g. --size 0.01"
                    ));
                    return Ok(());
                }
            };
            if v <= 0.0 || v > position_size {
                println!("{}", super::error_response(
                    &format!("--size must be > 0 and ≤ position size {}", position_size),
                    "INVALID_ARGUMENT",
                    &format!("Size must be between 0 and {}.", position_size)
                ));
                return Ok(());
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
        args.trigger_slippage,
    );

    // Compute worst-fill limit prices for display (matches what is sent on-chain)
    let closing_is_buy = !position_is_long;
    let trigger_multiplier = if closing_is_buy {
        1.0 + args.trigger_slippage / 100.0
    } else {
        1.0 - args.trigger_slippage / 100.0
    };
    let sl_limit_display = args.sl_px.map(|px| round_px(px * trigger_multiplier, sz_decimals));
    let tp_limit_display = args.tp_px.map(|px| round_px(px * trigger_multiplier, sz_decimals));

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

    let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "SIGNING_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "TX_SUBMIT_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };

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
