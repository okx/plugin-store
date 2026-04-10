use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct DepositLiquidityArgs {
    /// Market symbol or market token address (e.g. "ETH/USD" or 0x...)
    #[arg(long)]
    pub market: String,

    /// Long token amount in smallest units (e.g. ETH in wei). Use 0 to deposit short-side only.
    #[arg(long, default_value_t = 0)]
    pub long_amount: u128,

    /// Short token amount in smallest units (e.g. USDC units). Use 0 to deposit long-side only.
    #[arg(long, default_value_t = 0)]
    pub short_amount: u128,

    /// Minimum GM tokens to receive (slippage protection). Use 0 to accept any amount.
    #[arg(long, default_value_t = 0)]
    pub min_market_tokens: u128,

    /// Wallet address (defaults to logged-in wallet)
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(chain: &str, dry_run: bool, confirm: bool, args: DepositLiquidityArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    if args.long_amount == 0 && args.short_amount == 0 {
        anyhow::bail!("Must provide either --long-amount or --short-amount (or both).");
    }

    let wallet = args.from.clone().unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Fetch market info
    let markets = crate::api::fetch_markets(cfg).await?;
    let market = crate::api::find_market_by_symbol(&markets, &args.market)
        .ok_or_else(|| anyhow::anyhow!("Market '{}' not found on {}", args.market, chain))?;

    let market_token = market.market_token.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Market has no marketToken address"))?;
    let long_token = market.long_token.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Market has no longToken"))?;
    let short_token = market.short_token.as_deref()
        .ok_or_else(|| anyhow::anyhow!("Market has no shortToken"))?;

    let execution_fee = cfg.execution_fee_wei;

    // Approve long token if needed
    if !dry_run && args.long_amount > 0 {
        let allowance = crate::onchainos::check_allowance(
            cfg.rpc_url, long_token, &wallet, cfg.router,
        ).await.unwrap_or(0);
        if allowance < args.long_amount {
            eprintln!("WARNING: Approving {} long token to {} -- approving exact amount only. Use --dry-run to preview.", args.long_amount, cfg.router);
            let r = crate::onchainos::erc20_approve(
                cfg.chain_id, long_token, cfg.router, args.long_amount, Some(&wallet), false, confirm,
            ).await?;
            let approve_hash = crate::onchainos::extract_tx_hash(&r);
            eprintln!("Approval tx: {}", approve_hash);
            crate::onchainos::wait_for_tx(cfg.chain_id, approve_hash, &wallet, 60)?;
        }
    }

    // Approve short token if needed
    if !dry_run && args.short_amount > 0 {
        let allowance = crate::onchainos::check_allowance(
            cfg.rpc_url, short_token, &wallet, cfg.router,
        ).await.unwrap_or(0);
        if allowance < args.short_amount {
            eprintln!("WARNING: Approving {} short token to {} -- approving exact amount only. Use --dry-run to preview.", args.short_amount, cfg.router);
            let r = crate::onchainos::erc20_approve(
                cfg.chain_id, short_token, cfg.router, args.short_amount, Some(&wallet), false, confirm,
            ).await?;
            let approve_hash2 = crate::onchainos::extract_tx_hash(&r);
            eprintln!("Approval tx: {}", approve_hash2);
            crate::onchainos::wait_for_tx(cfg.chain_id, approve_hash2, &wallet, 60)?;
        }
    }

    // Build multicall: [sendWnt, (sendTokens long if > 0), (sendTokens short if > 0), createDeposit]
    let send_wnt = crate::abi::encode_send_wnt(cfg.deposit_vault, execution_fee);
    let create_deposit = crate::abi::encode_create_deposit(
        &wallet,
        "0x0000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000",
        market_token,
        long_token,
        short_token,
        args.min_market_tokens,
        execution_fee,
        cfg.chain_id,
    );

    let mut inner_calls = vec![send_wnt];
    if args.long_amount > 0 {
        inner_calls.push(crate::abi::encode_send_tokens(long_token, cfg.deposit_vault, args.long_amount));
    }
    if args.short_amount > 0 {
        inner_calls.push(crate::abi::encode_send_tokens(short_token, cfg.deposit_vault, args.short_amount));
    }
    inner_calls.push(create_deposit);

    let multicall_hex = crate::abi::encode_multicall(&inner_calls);
    let calldata = format!("0x{}", multicall_hex);

    eprintln!("=== Deposit Liquidity Preview ===");
    eprintln!("Market: {}", market.name.as_deref().unwrap_or("?"));
    eprintln!("Market token: {}", market_token);
    eprintln!("Long token amount: {}", args.long_amount);
    eprintln!("Short token amount: {}", args.short_amount);
    eprintln!("Min GM tokens to receive: {}", args.min_market_tokens);
    eprintln!("Execution fee: {} wei", execution_fee);
    eprintln!("⚠ GMX V2 keeper model: GM tokens minted 1-30s after tx lands.");
    eprintln!("Ask user to confirm before proceeding.");

    let result = crate::onchainos::wallet_contract_call_with_gas(
        cfg.chain_id,
        cfg.exchange_router,
        &calldata,
        Some(&wallet),
        Some(execution_fee),
        dry_run,
        confirm,
        Some(800_000),
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
            "longTokenAmount": args.long_amount.to_string(),
            "shortTokenAmount": args.short_amount.to_string(),
            "minGmTokens": args.min_market_tokens.to_string(),
            "executionFeeWei": execution_fee,
            "note": "GM tokens will be minted within 1-30s after tx confirmation by keeper",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
