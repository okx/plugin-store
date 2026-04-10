use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct ClosePositionArgs {
    /// Market token address of the position to close
    #[arg(long)]
    pub market_token: String,

    /// Collateral token address of the position
    #[arg(long)]
    pub collateral_token: String,

    /// Size to close in USD (use full position size for full close)
    #[arg(long)]
    pub size_usd: f64,

    /// Collateral amount to withdraw (use full collateral for full close)
    #[arg(long)]
    pub collateral_amount: u128,

    /// Is the position long?
    #[arg(long)]
    pub long: bool,

    /// Slippage in basis points (default: 100 = 1%)
    #[arg(long, default_value_t = 100)]
    pub slippage_bps: u32,

    /// Wallet address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(chain: &str, dry_run: bool, confirm: bool, args: ClosePositionArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.from.clone().unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Fetch current prices for acceptable price calculation
    let markets = crate::api::fetch_markets(cfg).await?;
    let tickers = crate::api::fetch_prices(cfg).await?;

    // Find market to get index token
    let market_info = markets.iter().find(|m| {
        m.market_token.as_deref()
            .map(|t| t.to_lowercase() == args.market_token.to_lowercase())
            .unwrap_or(false)
    });

    let price_tick = market_info
        .and_then(|m| m.index_token.as_deref())
        .and_then(|addr| crate::api::find_price(&tickers, addr));

    let (min_price_raw, max_price_raw) = price_tick
        .map(|t| (
            t.min_price.as_deref().unwrap_or("0").parse::<u128>().unwrap_or(0),
            t.max_price.as_deref().unwrap_or("0").parse::<u128>().unwrap_or(0),
        ))
        .unwrap_or((0, 0));

    let size_delta_usd = (args.size_usd * 1e30) as u128;

    // For close (decrease): acceptable price is inverted from open
    // long close: max_price * (1 + slippage) — we want to sell at or above this
    // short close: min_price * (1 - slippage)
    let base_price = if args.long { max_price_raw } else { min_price_raw };
    let acceptable_price = crate::abi::compute_acceptable_price(base_price, !args.long, args.slippage_bps);

    let execution_fee = cfg.execution_fee_wei;

    // Build multicall: [sendWnt, createOrder] (no sendTokens for decrease — collateral stays in vault)
    let send_wnt = crate::abi::encode_send_wnt(cfg.order_vault, execution_fee);
    let create_order = crate::abi::encode_create_order(
        &wallet,
        &wallet,
        &args.market_token,
        &args.collateral_token,
        4, // MarketDecrease
        size_delta_usd,
        args.collateral_amount,
        0, // triggerPrice = 0 for market orders
        acceptable_price,
        execution_fee,
        args.long,
        cfg.chain_id,
    );

    let multicall_hex = crate::abi::encode_multicall(&[send_wnt, create_order]);
    let calldata = format!("0x{}", multicall_hex);

    let token_infos = crate::api::fetch_tokens(cfg).await.unwrap_or_default();
    let index_decimals = market_info
        .and_then(|m| m.index_token.as_deref())
        .and_then(|addr| token_infos.iter().find(|t| t.address.as_deref().map(|a| a.to_lowercase()) == Some(addr.to_lowercase())))
        .and_then(|t| t.decimals)
        .unwrap_or(18u8);
    let mid_price_usd = if min_price_raw == 0 && max_price_raw == 0 {
        0.0
    } else {
        (crate::api::raw_price_to_usd(min_price_raw, index_decimals) + crate::api::raw_price_to_usd(max_price_raw, index_decimals)) / 2.0
    };

    eprintln!("=== Close Position Preview ===");
    eprintln!("Market token: {}", args.market_token);
    eprintln!("Direction: {}", if args.long { "LONG (closing)" } else { "SHORT (closing)" });
    eprintln!("Size to close: ${:.2} USD", args.size_usd);
    eprintln!("Collateral to withdraw: {} units", args.collateral_amount);
    eprintln!("Current price: ${:.4}", mid_price_usd);
    eprintln!("Acceptable price: {}", acceptable_price);
    eprintln!("⚠ GMX V2 keeper model: position closes 1-30s after tx lands.");
    eprintln!("Ask user to confirm before proceeding.");

    let result = crate::onchainos::wallet_contract_call(
        cfg.chain_id,
        cfg.exchange_router,
        &calldata,
        Some(&wallet),
        Some(execution_fee),
        dry_run,
        confirm,
    ).await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "dry_run": dry_run,
            "chain": chain,
            "txHash": tx_hash,
            "marketToken": args.market_token,
            "direction": if args.long { "long" } else { "short" },
            "sizeToClose_usd": args.size_usd,
            "collateralToWithdraw": args.collateral_amount.to_string(),
            "currentPrice_usd": format!("{:.4}", mid_price_usd),
            "acceptablePrice": acceptable_price.to_string(),
            "executionFeeWei": execution_fee,
            "note": "GMX V2 keeper model: position closes within 1-30s after tx confirmation",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
