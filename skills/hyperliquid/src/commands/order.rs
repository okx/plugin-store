use clap::Args;
use crate::api::{get_asset_index, get_all_mids};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{
    build_limit_order_action, build_market_order_action, submit_exchange_request,
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

    /// Limit price (required for limit orders, ignored for market orders)
    #[arg(long)]
    pub price: Option<String>,

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

    // Validate size is a valid number string
    let _size_check: f64 = args
        .size
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid size '{}' — must be a number (e.g. 0.01)", args.size))?;

    // Look up asset index
    let asset_idx = get_asset_index(info, &coin).await?;

    // Fetch current mid price for display
    let mids = get_all_mids(info).await?;
    let current_price = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Build action
    let action = match args.r#type.as_str() {
        "market" => build_market_order_action(asset_idx, is_buy, &args.size, args.reduce_only),
        "limit" => {
            let price_str = args
                .price
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("--price is required for limit orders"))?;
            // Validate price is a valid number
            let _price_check: f64 = price_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid price '{}' — must be a number", price_str))?;
            build_limit_order_action(asset_idx, is_buy, price_str, &args.size, args.reduce_only)
        }
        _ => anyhow::bail!("Unknown order type '{}'", args.r#type),
    };

    // Print preview regardless
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
                "reduceOnly": args.reduce_only,
                "currentMidPrice": current_price,
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

    // Resolve wallet
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Sign via onchainos
    let signed = onchainos_hl_sign(&action, nonce, &wallet, true, false)?;

    // Submit to exchange
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "coin": coin,
            "side": args.side,
            "size": args.size,
            "type": args.r#type,
            "result": result
        }))?
    );

    Ok(())
}
