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

    // Convert USD prices to GMX 30-decimal precision
    let trigger_price = (args.trigger_price_usd * 1e30) as u128;
    let acceptable_price = (args.acceptable_price_usd * 1e30) as u128;
    let size_delta_usd = (args.size_usd * 1e30) as u128;

    let execution_fee = cfg.execution_fee_wei;
    let order_type_u8 = args.order_type.to_u8();

    // Validate: for stop-loss on a long, trigger must be below current price
    let tickers = crate::api::fetch_prices(cfg).await.unwrap_or_default();
    if let Some(price_tick) = tickers.iter().find(|_| true) {
        // Simplified: just a placeholder for validation logic
        let _ = price_tick;
    }

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
    eprintln!("Trigger price: ${:.4}", args.trigger_price_usd);
    eprintln!("Acceptable price: ${:.4}", args.acceptable_price_usd);
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
            eprintln!("Approval tx: {}", crate::onchainos::extract_tx_hash(&approve_result));
        }
    }

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
