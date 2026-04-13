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

    let token_infos = crate::api::fetch_tokens(cfg).await.unwrap_or_default();
    let long_decimals = token_infos.iter()
        .find(|t| t.address.as_deref().map(|a| a.to_lowercase()) == Some(long_token.to_lowercase()))
        .and_then(|t| t.decimals).unwrap_or(18u8);
    let short_decimals = token_infos.iter()
        .find(|t| t.address.as_deref().map(|a| a.to_lowercase()) == Some(short_token.to_lowercase()))
        .and_then(|t| t.decimals).unwrap_or(6u8);
    let long_fmt = crate::api::format_token_amount(args.long_amount, long_decimals);
    let short_fmt = crate::api::format_token_amount(args.short_amount, short_decimals);
    let min_gm_fmt = crate::api::format_token_amount(args.min_market_tokens, 18);

    let execution_fee = cfg.execution_fee_wei;
    let execution_fee_eth = execution_fee as f64 / 1e18;

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

    // Pre-flight: token balance checks
    if args.long_amount > 0 {
        let bal = crate::rpc::check_erc20_balance(cfg.rpc_url, long_token, &wallet).await.unwrap_or(u128::MAX);
        if bal < args.long_amount {
            println!("{}", serde_json::to_string_pretty(&json!({
                "ok": false,
                "error": "INSUFFICIENT_LONG_TOKEN_BALANCE",
                "reason": "Wallet long token balance is less than --long-amount.",
                "token": long_token,
                "wallet_balance": bal.to_string(),
                "wallet_balance_formatted": crate::api::format_token_amount(bal, long_decimals),
                "required_amount": args.long_amount.to_string(),
                "required_formatted": long_fmt,
                "suggestion": format!("Reduce --long-amount to at most {} or top up the token.", bal)
            }))?);
            return Ok(());
        }
    }
    if args.short_amount > 0 {
        let bal = crate::rpc::check_erc20_balance(cfg.rpc_url, short_token, &wallet).await.unwrap_or(u128::MAX);
        if bal < args.short_amount {
            println!("{}", serde_json::to_string_pretty(&json!({
                "ok": false,
                "error": "INSUFFICIENT_SHORT_TOKEN_BALANCE",
                "reason": "Wallet short token balance is less than --short-amount.",
                "token": short_token,
                "wallet_balance": bal.to_string(),
                "wallet_balance_formatted": crate::api::format_token_amount(bal, short_decimals),
                "required_amount": args.short_amount.to_string(),
                "required_formatted": short_fmt,
                "suggestion": format!("Reduce --short-amount to at most {} or top up the token.", bal)
            }))?);
            return Ok(());
        }
    }

    // Approve long token if needed (only when about to execute)
    if confirm && !dry_run && args.long_amount > 0 {
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

    // Approve short token if needed (only when about to execute)
    if confirm && !dry_run && args.short_amount > 0 {
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
    eprintln!("Long token amount: {}", long_fmt);
    eprintln!("Short token amount: {}", short_fmt);
    eprintln!("Min GM tokens to receive: {}", min_gm_fmt);
    if args.min_market_tokens == 0 {
        eprintln!("⚠ min-market-tokens is 0 — no slippage protection on GM tokens received.");
    }
    eprintln!("Execution fee: {:.6} ETH", execution_fee_eth);
    eprintln!("⚠ GMX V2 keeper model: GM tokens minted 1-30s after tx lands.");
    if !confirm { eprintln!("Add --confirm to broadcast."); }

    if !confirm && !dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "status": "preview",
                "message": "Add --confirm to broadcast this transaction",
                "chain": chain,
                "market": market.name,
                "marketToken": market_token,
                "longTokenAmount": long_fmt,
                "shortTokenAmount": short_fmt,
                "minGmTokens": min_gm_fmt,
                "executionFee_eth": format!("{:.6}", execution_fee_eth),
                "calldata": calldata
            }))?
        );
        return Ok(());
    }

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
            "longTokenAmount": long_fmt,
            "shortTokenAmount": short_fmt,
            "minGmTokens": min_gm_fmt,
            "executionFee_eth": format!("{:.6}", execution_fee_eth),
            "note": "GM tokens will be minted within 1-30s after tx confirmation by keeper",
            "calldata": if dry_run { Some(calldata.as_str()) } else { None }
        }))?
    );
    Ok(())
}
