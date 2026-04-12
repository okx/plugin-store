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

    // size_delta_usd is in GMX's 10^30 USD precision
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

    // Pre-flight: ETH balance for execution fee + gas
    let eth_balance = crate::rpc::get_eth_balance(&wallet, cfg.rpc_url).await;
    let gas_margin: u128 = 200_000_000_000_000; // 0.0002 ETH
    let eth_required = execution_fee.saturating_add(gas_margin);
    if eth_balance < eth_required {
        println!("{}", serde_json::to_string_pretty(&json!({
            "ok": false,
            "error": "INSUFFICIENT_ETH_FOR_EXECUTION",
            "reason": "Wallet does not have enough ETH to cover execution fee + gas.",
            "eth_balance": format!("{:.8}", eth_balance as f64 / 1e18),
            "execution_fee_eth": format!("{:.8}", execution_fee as f64 / 1e18),
            "gas_buffer_eth": format!("{:.8}", gas_margin as f64 / 1e18),
            "eth_required": format!("{:.8}", eth_required as f64 / 1e18),
            "suggestion": format!("Top up wallet {} with at least {:.6} ETH.", wallet, (eth_required.saturating_sub(eth_balance)) as f64 / 1e18)
        }))?);
        return Ok(());
    }

    // Pre-flight: for increase orders, check collateral token balance
    if matches!(order_type_u8, 3 | 8) && args.collateral_amount > 0 {
        let bal = crate::rpc::check_erc20_balance(cfg.rpc_url, &args.collateral_token, &wallet).await.unwrap_or(u128::MAX);
        if bal < args.collateral_amount {
            println!("{}", serde_json::to_string_pretty(&json!({
                "ok": false,
                "error": "INSUFFICIENT_COLLATERAL_BALANCE",
                "reason": "Wallet collateral token balance is less than the required collateral amount.",
                "collateral_token": args.collateral_token,
                "wallet_balance": bal.to_string(),
                "required_amount": args.collateral_amount.to_string(),
                "suggestion": format!("Reduce --collateral-amount to at most {} or top up the collateral token.", bal)
            }))?);
            return Ok(());
        }
    }

    let execution_fee_eth = execution_fee as f64 / 1e18;
    eprintln!("=== Place Order Preview ===");
    eprintln!("Order type: {}", args.order_type.name());
    eprintln!("Market token: {}", args.market_token);
    eprintln!("Direction: {}", if args.long { "LONG" } else { "SHORT" });
    eprintln!("Size: ${:.2} USD", args.size_usd);
    eprintln!("Trigger price: ${:.4} (raw: {})", args.trigger_price_usd, trigger_price);
    eprintln!("Acceptable price: ${:.4} (raw: {})", args.acceptable_price_usd, acceptable_price);
    eprintln!("Index token decimals: {}", index_decimals);
    eprintln!("Current price: ${:.4}", current_price_usd);
    eprintln!("Execution fee: {:.6} ETH", execution_fee_eth);
    if !confirm { eprintln!("Add --confirm to broadcast."); }

    if !confirm && !dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "status": "preview",
                "message": "Add --confirm to broadcast this transaction",
                "chain": chain,
                "orderType": args.order_type.name(),
                "marketToken": args.market_token,
                "direction": if args.long { "long" } else { "short" },
                "sizeUsd": args.size_usd,
                "triggerPrice_usd": args.trigger_price_usd,
                "acceptablePrice_usd": args.acceptable_price_usd,
                "executionFee_eth": format!("{:.6}", execution_fee_eth),
                "calldata": calldata
            }))?
        );
        return Ok(());
    }

    // G19: snapshot existing order keys so we can diff after tx to find the new orderKey
    let orders_calldata = {
        let ds = cfg.datastore.trim_start_matches("0x");
        let wlt = wallet.trim_start_matches("0x");
        format!("0x42a6f8d3{:0>64}{:0>64}{:064x}{:064x}", ds, wlt, 0u128, 20u128)
    };
    let pre_keys: std::collections::HashSet<String> = if confirm && !dry_run {
        let raw = crate::rpc::eth_call(cfg.reader, &orders_calldata, cfg.rpc_url).await.unwrap_or_default();
        crate::commands::get_orders::extract_order_keys(&raw).into_iter().collect()
    } else {
        Default::default()
    };

    // For increase orders, check/approve collateral first
    if confirm && !dry_run && matches!(order_type_u8, 3 | 8) {
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

    // G19: after tx confirms, query orders and find the newly created orderKey
    let new_order_key: Option<String> = if confirm && !dry_run {
        crate::onchainos::wait_for_tx(cfg.chain_id, &tx_hash, &wallet, 60)?;
        let raw_after = crate::rpc::eth_call(cfg.reader, &orders_calldata, cfg.rpc_url).await.unwrap_or_default();
        let post_keys: std::collections::HashSet<String> = crate::commands::get_orders::extract_order_keys(&raw_after).into_iter().collect();
        post_keys.difference(&pre_keys).next().cloned()
    } else {
        None
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "dry_run": dry_run,
            "chain": chain,
            "txHash": tx_hash,
            "orderKey": new_order_key,
            "orderType": args.order_type.name(),
            "marketToken": args.market_token,
            "direction": if args.long { "long" } else { "short" },
            "sizeUsd": args.size_usd,
            "triggerPrice_usd": args.trigger_price_usd,
            "acceptablePrice_usd": args.acceptable_price_usd,
            "executionFee_eth": format!("{:.6}", execution_fee_eth),
            "note": "Order will be executed by keeper when trigger price is reached",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
