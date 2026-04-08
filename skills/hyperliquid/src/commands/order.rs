use clap::Args;
use crate::api::{get_asset_index, get_all_mids};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{
    build_bracketed_order_action, build_limit_order_action, build_market_order_action,
    format_px, submit_exchange_request,
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

pub async fn run(args: OrderArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    let coin = normalize_coin(&args.coin);
    let is_buy = args.side.to_lowercase() == "buy";
    let nonce = now_ms();

    let _size_check: f64 = args
        .size
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid size '{}' — must be a number (e.g. 0.01)", args.size))?;

    // TP/SL bracket validation
    if let Some(sl) = args.sl_px {
        if is_buy && args.tp_px.map_or(false, |tp| tp <= sl) {
            anyhow::bail!("Take-profit must be above stop-loss for a long position");
        }
        if !is_buy && args.tp_px.map_or(false, |tp| tp >= sl) {
            anyhow::bail!("Take-profit must be below stop-loss for a short position");
        }
    }

    let asset_idx = get_asset_index(info, &coin).await?;

    let mids = get_all_mids(info).await?;
    let current_price = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let has_bracket = args.sl_px.is_some() || args.tp_px.is_some();

    // Build the entry order element (without the wrapper)
    let action = if has_bracket {
        let entry_element = match args.r#type.as_str() {
            "market" => serde_json::json!({
                "a": asset_idx,
                "b": is_buy,
                "p": "0",
                "s": args.size,
                "r": args.reduce_only,
                "t": {
                    "trigger": {
                        "isMarket": true,
                        "tpsl": "tp",
                        "triggerPx": "0"
                    }
                }
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
                    "s": args.size,
                    "r": args.reduce_only,
                    "t": { "limit": { "tif": "Gtc" } }
                })
            }
            _ => anyhow::bail!("Unknown order type '{}'", args.r#type),
        };

        let sl_px_str = args.sl_px.map(format_px);
        let tp_px_str = args.tp_px.map(format_px);

        build_bracketed_order_action(
            entry_element,
            asset_idx,
            is_buy,
            &args.size,
            sl_px_str.as_deref(),
            tp_px_str.as_deref(),
        )
    } else {
        match args.r#type.as_str() {
            "market" => build_market_order_action(asset_idx, is_buy, &args.size, args.reduce_only),
            "limit" => {
                let price_str = args
                    .price
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("--price is required for limit orders"))?;
                let _: f64 = price_str
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid price '{}'", price_str))?;
                build_limit_order_action(asset_idx, is_buy, price_str, &args.size, args.reduce_only)
            }
            _ => anyhow::bail!("Unknown order type '{}'", args.r#type),
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "coin": coin,
                "assetIndex": asset_idx,
                "side": args.side,
                "size": args.size,
                "type": args.r#type,
                "price": args.price,
                "stopLoss": args.sl_px.map(format_px),
                "takeProfit": args.tp_px.map(format_px),
                "reduceOnly": args.reduce_only,
                "currentMidPrice": current_price,
                "grouping": if has_bracket { "normalTpsl" } else { "na" },
                "nonce": nonce
            },
            "action": action
        }))?
    );

    if args.dry_run {
        println!("\n[DRY RUN] Order not signed or submitted.");
        return Ok(());
    }

    if !args.confirm {
        println!("\n[PREVIEW] Add --confirm to sign and submit this order.");
        println!("WARNING: This will place a real perpetual order on Hyperliquid.");
        println!("         Perpetuals trading involves significant risk including total loss.");
        return Ok(());
    }

    let wallet = resolve_wallet(CHAIN_ID)?;
    let signed = onchainos_hl_sign(&action, nonce, &wallet, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "coin": coin,
            "side": args.side,
            "size": args.size,
            "type": args.r#type,
            "stopLoss": args.sl_px.map(format_px),
            "takeProfit": args.tp_px.map(format_px),
            "result": result
        }))?
    );

    Ok(())
}
