use clap::Args;
use crate::api::{
    fetch_perp_dexs, get_all_mids_for_dex, get_asset_meta_for_coin,
    get_clearinghouse_state_for_dex, parse_coin,
};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, report_plugin_info, resolve_wallet};
use crate::signing::{build_close_action, round_px, submit_exchange_request};

#[derive(Args)]
pub struct CloseArgs {
    /// Coin whose position to close (e.g. BTC, ETH, SOL)
    #[arg(long)]
    pub coin: String,

    /// Close only this many base units instead of the entire position
    #[arg(long)]
    pub size: Option<String>,

    /// Slippage tolerance for the market close order, in percent (default 5.0 = 5%)
    #[arg(long, default_value = "5.0")]
    pub slippage: f64,

    /// Dry run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,

    /// Optional strategy ID tag for attribution. All closes are reported to the OKX
    /// backend regardless; this flag just attaches a strategy label. Empty if omitted.
    #[arg(long)]
    pub strategy_id: Option<String>,
}

pub async fn run(args: CloseArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    // HIP-3: parse dex prefix; coin variable keeps full prefixed form for builder DEX
    // (e.g. "xyz:CL") so position match below works against HIP-3 position.coin field.
    let (dex_opt, _) = parse_coin(&args.coin);
    let coin = if dex_opt.is_some() {
        let (d, b) = parse_coin(&args.coin);
        format!("{}:{}", d.unwrap(), b.to_uppercase())
    } else {
        normalize_coin(&args.coin)
    };
    let nonce = now_ms();

    // Look up asset index and sz_decimals (dex-aware via registry)
    let registry = fetch_perp_dexs(info).await.unwrap_or_default();
    let (asset_idx, sz_decimals) = match get_asset_meta_for_coin(info, &coin, &registry).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry. If using a builder DEX coin (e.g. xyz:CL), run `hyperliquid-plugin dex-list`."));
            return Ok(());
        }
    };

    // Resolve wallet
    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };

    // Fetch current position to determine direction and full size (dex-aware)
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
    for pw in positions {
        let pos = &pw["position"];
        if pos["coin"].as_str().map(|c| c.to_uppercase()) == Some(coin.to_uppercase()) {
            if let Some(s) = pos["szi"].as_str() {
                position_szi = s.parse().ok();
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

    // Determine close size
    let close_size = match &args.size {
        Some(s) => {
            let v: f64 = match s.parse() {
                Ok(v) => v,
                Err(_) => {
                    println!("{}", super::error_response(&format!("Invalid size '{}' — must be a number", s), "INVALID_ARGUMENT", "Provide a numeric size value, e.g. --size 0.01"));
                    return Ok(());
                }
            };
            if v <= 0.0 {
                println!("{}", super::error_response("Close size must be positive", "INVALID_ARGUMENT", "Provide a positive close size value."));
                return Ok(());
            }
            if v > position_size {
                println!("{}", super::error_response(
                    &format!("Close size {} exceeds position size {}", v, position_size),
                    "INVALID_ARGUMENT",
                    &format!("Maximum close size is {}.", position_size)
                ));
                return Ok(());
            }
            s.clone()
        }
        None => format!("{}", position_size),
    };

    // Fetch current price — must use the per-DEX mids endpoint when the coin lives on a
    // builder DEX (HIP-3). Default-DEX get_all_mids only returns BTC/ETH/SOL/etc.; an
    // xyz:CL or cash:HOOD lookup against it misses, mid_f becomes 0.0, slippage_px is 0,
    // and HL rejects the close because worst_fill_price=0 is invalid.
    let mids = match get_all_mids_for_dex(info, dex_opt.as_deref()).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
    let current_price = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let closing_side = if position_is_long { "sell" } else { "buy" };
    let close_is_buy = !position_is_long;
    let mid_f = current_price.parse::<f64>().unwrap_or(0.0);
    let slippage_multiplier = if close_is_buy { 1.0 + args.slippage / 100.0 } else { 1.0 - args.slippage / 100.0 };
    let slippage_px_str = round_px(mid_f * slippage_multiplier, sz_decimals);

    let action = build_close_action(asset_idx, position_is_long, &close_size, &slippage_px_str);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "coin": coin,
                "positionSide": position_side,
                "positionSize": position_size,
                "closingSize": close_size,
                "closingSide": closing_side,
                "currentMidPrice": current_price,
                "type": "market",
                "slippagePct": args.slippage,
                "worstFillPrice": slippage_px_str,
                "reduceOnly": true,
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
        eprintln!("\n[PREVIEW] Add --confirm to sign and market-close this position.");
        eprintln!("WARNING: Market orders execute immediately at prevailing price.");
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

    let statuses = result["response"]["data"]["statuses"]
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let avg_px = statuses["filled"]["avgPx"].as_str().map(|s| s.to_string());
    let oid = statuses["filled"]["oid"]
        .as_u64()
        .or_else(|| statuses["resting"]["oid"].as_u64());

    // Attribution: report every close that produced an oid.
    // strategy_id is optional — defaults to empty string so the backend still receives a record.
    if let Some(oid_val) = oid {
        let ts_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let sid = args.strategy_id.as_deref().unwrap_or("");
        let report_payload = serde_json::json!({
            "wallet": wallet,
            "proxyAddress": "",
            "order_id": oid_val.to_string(),
            "tx_hashes": [],
            "market_id": coin,
            "asset_id": "",
            "side": closing_side.to_uppercase(),
            "amount": close_size,
            "symbol": "USDC",
            "price": avg_px.clone().unwrap_or_default(),
            "timestamp": ts_now,
            "strategy_id": sid,
            "plugin_name": "hyperliquid-plugin",
        });
        if let Err(e) = report_plugin_info(&report_payload) {
            eprintln!("[hyperliquid] Warning: report-plugin-info failed: {}", e);
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "action": "close",
            "coin": coin,
            "side": closing_side,
            "size": close_size,
            "result": result
        }))?
    );

    Ok(())
}
