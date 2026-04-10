use clap::Args;
use serde_json::json;

/// Order type for CLI
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OrderType {
    /// Limit increase (entry limit order)
    LimitIncrease,
    /// Limit decrease (take profit)
    LimitDecrease,
    /// Stop-loss decrease
    StopLoss,
    /// Stop increase
    StopIncrease,
}

impl OrderType {
    pub fn to_u8(&self) -> u8 {
        match self {
            OrderType::LimitIncrease => 3,
            OrderType::LimitDecrease => 5,
            OrderType::StopLoss => 6,
            OrderType::StopIncrease => 8,
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            OrderType::LimitIncrease => "LimitIncrease",
            OrderType::LimitDecrease => "LimitDecrease",
            OrderType::StopLoss => "StopLossDecrease",
            OrderType::StopIncrease => "StopIncrease",
        }
    }
}

#[derive(Args)]
pub struct PlaceOrderArgs {
    /// Order type: limit-increase, limit-decrease, stop-loss, stop-increase
    #[arg(long, value_enum)]
    pub order_type: OrderType,

    /// Market token address
    #[arg(long)]
    pub market_token: String,

    /// Collateral token address
    #[arg(long)]
    pub collateral_token: String,

    /// Position size in USD
    #[arg(long)]
    pub size_usd: f64,

    /// Collateral amount in smallest units
    #[arg(long)]
    pub collateral_amount: u128,

    /// Trigger price in USD (e.g. 1700.0 for $1700)
    #[arg(long)]
    pub trigger_price_usd: f64,

    /// Acceptable price in USD (use same as trigger or add slippage buffer)
    #[arg(long)]
    pub acceptable_price_usd: f64,

    /// Is this for a long position?
    #[arg(long)]
    pub long: bool,

    /// Wallet address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(chain: &str, dry_run: bool, confirm: bool, args: PlaceOrderArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.from.clone().unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let execution_fee = cfg.execution_fee_wei;
    let order_type_u8 = args.order_type.to_u8();

    // Look up index token decimals so we can convert USD → raw GMX price format.
    // GMX price format: price_usd × 10^(30 - token_decimals)
    //   BTC (8 dec)  → × 10^22
    //   ETH (18 dec) → × 10^12
    let markets = crate::api::fetch_markets(cfg).await?;
    let token_infos = crate::api::fetch_tokens(cfg).await.unwrap_or_default();
    let market_info = markets.iter().find(|m| {
        m.market_token.as_deref()
            .map(|t| t.to_lowercase() == args.market_token.to_lowercase())
            .unwrap_or(false)
    });
    let index_decimals = market_info
        .and_then(|m| m.index_token.as_deref())
        .and_then(|addr| token_infos.iter().find(|t|
            t.address.as_deref().map(|a| a.to_lowercase()) == Some(addr.to_lowercase())
        ))
        .and_then(|t| t.decimals)
        .unwrap_or(18u8);
    let price_exponent = 30u32 - index_decimals as u32;
    let price_precision: u128 = 10u128.pow(price_exponent);

    // Convert USD price to raw GMX format using integer math to avoid f64 precision loss.
    let usd_to_raw = |usd: f64| -> u128 {
        let int_part = usd.floor() as u128;
        let frac_part = usd - usd.floor();
        int_part * price_precision + (frac_part * 10f64.powi(price_exponent as i32)) as u128
    };
    let trigger_price = usd_to_raw(args.trigger_price_usd);
    let acceptable_price = usd_to_raw(args.acceptable_price_usd);

    // size_delta_usd is in GMX's 10^30 USD precision (not token-decimal-adjusted)
    let size_delta_usd = {
        let int_part = args.size_usd.floor() as u128;
        let frac_part = args.size_usd - args.size_usd.floor();
        let usd_precision: u128 = 1_000_000_000_000_000_000_000_000_000_000;
        int_part * usd_precision + (frac_part * 1e30) as u128
    };

    // Fetch current price for display
    let tickers = crate::api::fetch_prices(cfg).await.unwrap_or_default();
    let current_price_usd = market_info
        .and_then(|m| m.index_token.as_deref())
        .and_then(|addr| crate::api::find_price(&tickers, addr))
        .map(|t| {
            let raw = t.min_price.as_deref().unwrap_or("0").parse::<u128>().unwrap_or(0);
            crate::api::raw_price_to_usd(raw, index_decimals)
        })
        .unwrap_or(0.0);

    // Build multicall: [sendWnt, (sendTokens if increase order), createOrder]
    let send_wnt = crate::abi::encode_send_wnt(cfg.order_vault, execution_fee);
    let create_order = crate::abi::encode_create_order(
        &wallet,
        &wallet,
        &args.market_token,
        &args.collateral_token,
        order_type_u8,
        size_delta_usd,
        args.collateral_amount,
        trigger_price,
        acceptable_price,
        execution_fee,
        args.long,
        cfg.chain_id,
    );

    let inner_calls = match order_type_u8 {
        // Increase orders also need sendTokens
        3 | 8 => {
            let send_tokens = crate::abi::encode_send_tokens(
                &args.collateral_token,
                cfg.order_vault,
                args.collateral_amount,
            );
            vec![send_wnt, send_tokens, create_order]
        }
        _ => vec![send_wnt, create_order],
    };

    let multicall_hex = crate::abi::encode_multicall(&inner_calls);
    let calldata = format!("0x{}", multicall_hex);

    eprintln!("=== Place Order Preview ===");
    eprintln!("Order type: {}", args.order_type.name());
    eprintln!("Market token: {}", args.market_token);
    eprintln!("Direction: {}", if args.long { "LONG" } else { "SHORT" });
    eprintln!("Size: ${:.2} USD", args.size_usd);
    eprintln!("Trigger price: ${:.4} (raw: {})", args.trigger_price_usd, trigger_price);
    eprintln!("Acceptable price: ${:.4} (raw: {})", args.acceptable_price_usd, acceptable_price);
    eprintln!("Index token decimals: {}", index_decimals);
    eprintln!("Current price: ${:.4}", current_price_usd);
    eprintln!("Execution fee: {} wei", execution_fee);
    eprintln!("Ask user to confirm before proceeding.");

    // For increase orders, check/approve collateral first
    if !dry_run && matches!(order_type_u8, 3 | 8) {
        let allowance = crate::onchainos::check_allowance(
            cfg.rpc_url,
            &args.collateral_token,
            &wallet,
            cfg.router,
        ).await.unwrap_or(0);
        if allowance < args.collateral_amount {
            eprintln!("WARNING: Approving {} collateral token to {} -- approving exact amount only. Use --dry-run to preview.", args.collateral_amount, cfg.router);
            let approve_result = crate::onchainos::erc20_approve(
                cfg.chain_id,
                &args.collateral_token,
                cfg.router,
                args.collateral_amount,
                Some(&wallet),
                false,
                confirm,
            ).await?;
            let approve_hash = crate::onchainos::extract_tx_hash(&approve_result);
            eprintln!("Approval tx: {}", approve_hash);
            crate::onchainos::wait_for_tx(cfg.chain_id, approve_hash, &wallet, 60)?;
        }
    }

    let result = crate::onchainos::wallet_contract_call_with_gas(
        cfg.chain_id,
        cfg.exchange_router,
        &calldata,
        Some(&wallet),
        Some(execution_fee),
        dry_run,
        confirm,
        Some(500_000),
    ).await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "dry_run": dry_run,
            "chain": chain,
            "txHash": tx_hash,
            "orderType": args.order_type.name(),
            "marketToken": args.market_token,
            "direction": if args.long { "long" } else { "short" },
            "sizeUsd": args.size_usd,
            "triggerPrice_usd": args.trigger_price_usd,
            "acceptablePrice_usd": args.acceptable_price_usd,
            "executionFeeWei": execution_fee,
            "note": "Order will be executed by keeper when trigger price is reached",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
