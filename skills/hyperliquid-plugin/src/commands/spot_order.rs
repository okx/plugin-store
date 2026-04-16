use clap::Args;
use crate::api::{get_spot_asset_meta, get_all_mids};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{
    build_limit_order_action, build_market_order_action,
    round_px, submit_exchange_request,
};

#[derive(Args)]
pub struct SpotOrderArgs {
    /// Base token to trade (e.g. PURR, HYPE)
    #[arg(long)]
    pub coin: String,

    /// Side: buy or sell
    #[arg(long, value_parser = ["buy", "sell"])]
    pub side: String,

    /// Size in base token units (e.g. 100 for 100 PURR)
    #[arg(long)]
    pub size: String,

    /// Order type: market or limit
    #[arg(long, value_parser = ["market", "limit"], default_value = "market")]
    pub r#type: String,

    /// Limit price in USDC (required for limit orders)
    #[arg(long)]
    pub price: Option<String>,

    /// Slippage tolerance in percent for market orders (default: 5.0).
    /// E.g. --slippage 1.0 allows at most 1% worse than mid price.
    #[arg(long, default_value = "5.0")]
    pub slippage: f64,

    /// Post-only (limit orders only) — order is cancelled instead of crossing the spread;
    /// ensures maker rebate. Uses Hyperliquid's ALO (Add Liquidity Only) TIF.
    #[arg(long)]
    pub post_only: bool,

    /// Dry run — show payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: SpotOrderArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    let coin = normalize_coin(&args.coin);
    let is_buy = args.side.to_lowercase() == "buy";
    let nonce = now_ms();

    // Validate inputs
    let _size_check: f64 = args
        .size
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid --size '{}' — must be a number (e.g. 100)", args.size))?;

    if args.post_only && args.r#type != "limit" {
        anyhow::bail!("--post-only requires --type limit");
    }
    if args.slippage <= 0.0 || args.slippage > 100.0 {
        anyhow::bail!("--slippage must be between 0 and 100 (got {})", args.slippage);
    }

    // Pre-flight: check notional value against HL's 10 USDC spot minimum
    if let (Some(price_str), Ok(size_f)) = (args.price.as_deref(), args.size.parse::<f64>()) {
        if let Ok(price_f) = price_str.parse::<f64>() {
            let notional = size_f * price_f;
            if notional < 10.0 {
                anyhow::bail!(
                    "Order value {:.4} USDC is below Hyperliquid's 10 USDC minimum for spot orders. \
                     Increase --size or --price (current: {} × {} = {:.4} USDC).",
                    notional, args.size, price_str, notional
                );
            }
        }
    }

    // Look up spot asset — returns (order_asset_idx, raw_market_idx, sz_decimals)
    let (asset_idx, market_idx, sz_decimals) = get_spot_asset_meta(info, &coin).await?;

    // Current mid price (keyed as "@{market_idx}" in allMids)
    let mids = get_all_mids(info).await?;
    let price_key = format!("@{}", market_idx);
    let current_price = mids
        .get(&price_key)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mid_f = current_price.parse::<f64>().unwrap_or(0.0);
    // Compute slippage price using configurable tolerance (default 5%)
    let multiplier = if is_buy { 1.0 + args.slippage / 100.0 } else { 1.0 - args.slippage / 100.0 };
    let slippage_px_str = round_px(mid_f * multiplier, sz_decimals);

    // Spot orders always have reduce_only = false (no position to reduce)
    let action = match args.r#type.as_str() {
        "market" => build_market_order_action(asset_idx, is_buy, &args.size, false, &slippage_px_str),
        "limit" => {
            let price_str = args
                .price
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("--price is required for limit orders"))?;
            let _: f64 = price_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid --price '{}'", price_str))?;
            let tif = if args.post_only { "Alo" } else { "Gtc" };
            build_limit_order_action(asset_idx, is_buy, price_str, &args.size, false, tif)
        }
        _ => anyhow::bail!("Unknown order type '{}'", args.r#type),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "market": "spot",
                "coin": coin,
                "pair": format!("{}/USDC", coin),
                "assetIndex": asset_idx,
                "side": args.side,
                "size": args.size,
                "type": args.r#type,
                "price": args.price,
                "currentMidPrice": current_price,
                "slippagePct": args.slippage,
                "postOnly": args.post_only,
                "worstFillPrice": if args.r#type == "market" { Some(slippage_px_str.clone()) } else { None },
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
        eprintln!("\n[PREVIEW] Add --confirm to sign and submit this spot order.");
        eprintln!("WARNING: This will place a real spot order on Hyperliquid.");
        return Ok(());
    }

    let wallet = resolve_wallet(CHAIN_ID)?;
    let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "market": "spot",
            "coin": coin,
            "side": args.side,
            "size": args.size,
            "type": args.r#type,
            "price": args.price,
            "result": result
        }))?
    );

    Ok(())
}
