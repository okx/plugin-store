use clap::Args;
use serde_json::json;

/// Convert a USD float to GMX 30-decimal u128 without floating-point precision loss.
fn parse_usd_to_u128(val: f64) -> u128 {
    // Split into integer and fractional parts to avoid (val * 1e30) overflow/imprecision
    let integer_part = val.floor() as u128;
    let frac_part = val - val.floor();
    let precision: u128 = 1_000_000_000_000_000_000_000_000_000_000; // 10^30
    integer_part * precision + (frac_part * 1e30) as u128
}

#[derive(Args)]
pub struct OpenPositionArgs {
    /// Market symbol or index token address (e.g. "ETH" or "ETH/USD")
    #[arg(long)]
    pub market: String,

    /// Collateral token address (e.g. USDC on Arbitrum: 0xaf88d065e77c8cC2239327C5EDb3A432268e5831)
    #[arg(long)]
    pub collateral_token: String,

    /// Collateral amount in smallest units (e.g. 1000000000 for 1000 USDC with 6 decimals)
    #[arg(long)]
    pub collateral_amount: u128,

    /// Position size in USD (e.g. 5000.0 for $5000 leveraged position)
    #[arg(long)]
    pub size_usd: f64,

    /// Long (true) or short (false)
    #[arg(long)]
    pub long: bool,

    /// Slippage in basis points (default: 100 = 1%)
    #[arg(long, default_value_t = 100)]
    pub slippage_bps: u32,

    /// Wallet address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(chain: &str, dry_run: bool, confirm: bool, args: OpenPositionArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.from.clone().unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Fetch markets to find the target market
    let markets = crate::api::fetch_markets(cfg).await?;
    let market = crate::api::find_market_by_symbol(&markets, &args.market)
        .ok_or_else(|| anyhow::anyhow!("Market '{}' not found on {}", args.market, chain))?;

    let market_token = market.market_token.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Market has no marketToken address"))?;
    let index_token = market.index_token.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Market has no indexToken (swap-only market?)"))?;

    // Fetch prices
    let tickers = crate::api::fetch_prices(cfg).await?;
    let price_tick = crate::api::find_price(&tickers, index_token)
        .ok_or_else(|| anyhow::anyhow!("Price not found for index token {}", index_token))?;

    let min_price_raw: u128 = price_tick.min_price.as_deref().unwrap_or("0").parse().unwrap_or(0);
    let max_price_raw: u128 = price_tick.max_price.as_deref().unwrap_or("0").parse().unwrap_or(0);
    // GMX prices are stored as price_usd * 10^(30 - token_decimals).
    // For display, fetch token decimals to convert properly. Default to 18.
    let token_infos = crate::api::fetch_tokens(cfg).await.unwrap_or_default();
    let index_decimals = token_infos.iter()
        .find(|t| t.address.as_deref().map(|a| a.to_lowercase()) == Some(index_token.to_lowercase()))
        .and_then(|t| t.decimals)
        .unwrap_or(18u8);
    let min_price_usd = crate::api::raw_price_to_usd(min_price_raw, index_decimals);
    let max_price_usd = crate::api::raw_price_to_usd(max_price_raw, index_decimals);
    let mid_price_usd = (min_price_usd + max_price_usd) / 2.0;

    // Size in GMX 30-decimal units
    let size_delta_usd = parse_usd_to_u128(args.size_usd);

    // Check liquidity
    let avail_liq = if args.long {
        market.available_liquidity_long.as_deref().unwrap_or("0").parse::<u128>().unwrap_or(0)
    } else {
        market.available_liquidity_short.as_deref().unwrap_or("0").parse::<u128>().unwrap_or(0)
    };
    let avail_liq_usd = avail_liq as f64 / 1e30;
    if size_delta_usd > avail_liq {
        anyhow::bail!(
            "Insufficient liquidity. Required: ${:.2} USD, Available: ${:.2} USD",
            args.size_usd,
            avail_liq_usd
        );
    }

    // Compute acceptable price with slippage
    let base_price = if args.long { min_price_raw } else { max_price_raw };
    let acceptable_price = crate::abi::compute_acceptable_price(base_price, args.long, args.slippage_bps);

    let execution_fee = cfg.execution_fee_wei;

    // Check ERC-20 allowance and approve if needed.
    // Guard with `confirm` so approve and the main TX are always in sync:
    // without --confirm neither fires; with --confirm both fire.
    if !dry_run {
        let allowance = crate::onchainos::check_allowance(
            cfg.rpc_url,
            &args.collateral_token,
            &wallet,
            cfg.router,
        ).await.unwrap_or(0);

        if allowance < args.collateral_amount {
            if !confirm {
                eprintln!("NOTE: Allowance insufficient ({} < {}). Re-run with --confirm to approve and open position.",
                    allowance, args.collateral_amount);
            } else {
                eprintln!("Approving {} collateral token to router {}...", args.collateral_amount, cfg.router);
                let approve_result = crate::onchainos::erc20_approve(
                    cfg.chain_id,
                    &args.collateral_token,
                    cfg.router,
                    args.collateral_amount,
                    Some(&wallet),
                    false,
                    true,
                ).await?;
                let approve_hash = crate::onchainos::extract_tx_hash(&approve_result);
                eprintln!("Approval tx: {}", approve_hash);
            }
        }
    }

    // Build multicall: [sendWnt, sendTokens, createOrder]
    let send_wnt = crate::abi::encode_send_wnt(cfg.order_vault, execution_fee);
    let send_tokens = crate::abi::encode_send_tokens(
        &args.collateral_token,
        cfg.order_vault,
        args.collateral_amount,
    );
    let create_order = crate::abi::encode_create_order(
        &wallet,
        &wallet,
        market_token,
        &args.collateral_token,
        2, // MarketIncrease
        size_delta_usd,
        args.collateral_amount,
        0, // triggerPrice = 0 for market orders
        acceptable_price,
        execution_fee,
        args.long,
        cfg.chain_id,
    );

    let multicall_hex = crate::abi::encode_multicall(&[send_wnt, send_tokens, create_order]);
    let calldata = format!("0x{}", multicall_hex);

    let leverage = if mid_price_usd > 0.0 {
        args.size_usd / (args.collateral_amount as f64 / 1e6)
    } else {
        0.0
    };

    // Preview
    eprintln!("=== Open Position Preview ===");
    eprintln!("Market: {}", market.name.as_deref().unwrap_or("?"));
    eprintln!("Direction: {}", if args.long { "LONG" } else { "SHORT" });
    eprintln!("Size: ${:.2} USD", args.size_usd);
    eprintln!("Collateral: {} units", args.collateral_amount);
    eprintln!("Current price: ${:.4}", mid_price_usd);
    eprintln!("Acceptable price: {}", acceptable_price);
    eprintln!("Execution fee: {} wei", execution_fee);
    eprintln!("Estimated leverage: {:.1}x", leverage);
    eprintln!("⚠ GMX V2 uses a keeper model — position opens 1-30s after tx lands.");
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
            "market": market.name,
            "marketToken": market_token,
            "direction": if args.long { "long" } else { "short" },
            "sizeDeltaUsd": args.size_usd,
            "collateralAmount": args.collateral_amount.to_string(),
            "entryPrice_approx_usd": format!("{:.4}", mid_price_usd),
            "acceptablePrice": acceptable_price.to_string(),
            "executionFeeWei": execution_fee,
            "note": "GMX V2 keeper model: position will open within 1-30s after tx confirmation",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
