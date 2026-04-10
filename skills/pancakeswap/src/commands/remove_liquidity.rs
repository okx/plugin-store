/// `pancakeswap remove-liquidity` — decrease liquidity + collect (two-step).

use anyhow::Result;

pub struct RemoveLiquidityArgs {
    pub token_id: u128,
    pub liquidity_pct: f64,   // 0–100, percentage of position liquidity to remove
    pub slippage: f64,        // slippage tolerance in percent (e.g. 0.5 = 0.5%)
    pub chain: u64,
    pub dry_run: bool,
    pub confirm: bool,
}

pub async fn run(args: RemoveLiquidityArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    // Fetch current position data to verify it exists and get liquidity
    println!("Fetching position #{} on chain {}...", args.token_id, args.chain);
    let pos = crate::rpc::get_position(cfg.npm, args.token_id, cfg.rpc_url).await?;

    if pos.liquidity == 0 && !args.dry_run {
        anyhow::bail!("Position #{} has zero liquidity. Nothing to remove.", args.token_id);
    }
    // In dry-run mode with zero liquidity, use a synthetic value to preview calldata
    let effective_liquidity = if pos.liquidity == 0 && args.dry_run { 1_000_000u128 } else { pos.liquidity };

    let sym0 = crate::rpc::get_symbol(&pos.token0, cfg.rpc_url).await.unwrap_or_else(|_| pos.token0.clone());
    let sym1 = crate::rpc::get_symbol(&pos.token1, cfg.rpc_url).await.unwrap_or_else(|_| pos.token1.clone());

    let liquidity_to_remove = (effective_liquidity as f64 * args.liquidity_pct / 100.0) as u128;

    // Bug 3 fix: compute actual token amounts from V3 liquidity math using the current
    // pool price, instead of the incorrect tokens_owed proxy used previously.
    // tokens_owed represents already-accrued fees credited to the position — it is
    // completely unrelated to the amounts returned by decreaseLiquidity, which are
    // derived from the position's liquidity and the current sqrtPrice.
    let pool = crate::rpc::get_pool_address(cfg.factory, &pos.token0, &pos.token1, pos.fee, cfg.rpc_url).await?;
    let (sqrt_price_x96, tick_current) = crate::rpc::get_slot0(&pool, cfg.rpc_url).await?;
    let (amount0_out, amount1_out) = crate::rpc::amounts_from_liquidity(
        sqrt_price_x96,
        pos.tick_lower,
        pos.tick_upper,
        tick_current,
        liquidity_to_remove,
    );

    let slippage_bps = (args.slippage * 100.0) as u128;
    let amount0_min = amount0_out.saturating_mul(10000 - slippage_bps) / 10000;
    let amount1_min = amount1_out.saturating_mul(10000 - slippage_bps) / 10000;

    println!("Remove Liquidity (chain {}):", args.chain);
    println!("  Position:     #{}", args.token_id);
    println!("  Pair:         {}/{}", sym0, sym1);
    println!("  Current tick: {} (pool sqrtPriceX96: {})", tick_current, sqrt_price_x96);
    println!("  Total liq:    {}{}", effective_liquidity, if pos.liquidity == 0 && args.dry_run { " [synthetic for dry-run]" } else { "" });
    println!("  Remove:       {}% = {}", args.liquidity_pct, liquidity_to_remove);
    println!("  Tick range:   {} to {}", pos.tick_lower, pos.tick_upper);
    println!("  Expected out: {} {} / {} {} (before slippage)", amount0_out, sym0, amount1_out, sym1);
    println!("  Min out:      {} {} / {} {} ({}% slippage)", amount0_min, sym0, amount1_min, sym1, args.slippage);
    println!("  Owed fees:    {} {} / {} {}", pos.tokens_owed0, sym0, pos.tokens_owed1, sym1);
    println!("  NPM:          {}", cfg.npm);

    // Deadline: 20 minutes from now
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() + 1200)
        .unwrap_or(9_999_999_999);

    // Fetch wallet address for use as collect recipient
    let wallet_address = if args.dry_run {
        "0x0000000000000000000000000000000000000001".to_string()
    } else {
        crate::onchainos::get_wallet_address().await?
    };

    // Step 1: decreaseLiquidity
    println!("\nStep 1: Calling decreaseLiquidity...");
    println!("  amount0Min: {} (slippage {}%)", amount0_min, args.slippage);
    println!("  amount1Min: {} (slippage {}%)", amount1_min, args.slippage);
    let decrease_calldata = crate::calldata::encode_decrease_liquidity(
        args.token_id,
        liquidity_to_remove,
        amount0_min,
        amount1_min,
        deadline,
    )?;

    if args.dry_run {
        println!("  [dry-run] onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, cfg.npm, decrease_calldata);
    } else {
        let r = crate::onchainos::wallet_contract_call(args.chain, cfg.npm, &decrease_calldata, None, None, args.dry_run, args.confirm).await?;
        println!("  decreaseLiquidity tx: {}", crate::onchainos::extract_tx_hash(&r));
        // Wait for nonce to settle before collect
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Step 2: collect — MUST always follow decreaseLiquidity
    // Note: decreaseLiquidity credits tokens to the position but does NOT transfer them.
    // collect transfers the credited tokens to the recipient.
    println!("\nStep 2: Calling collect to transfer tokens to wallet...");
    println!("  Recipient: {}", wallet_address);
    let collect_calldata = crate::calldata::encode_collect(
        args.token_id,
        &wallet_address,
    )?;

    if args.dry_run {
        println!("  [dry-run] onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, cfg.npm, collect_calldata);
        println!("\nDry-run complete. No transactions submitted.");
        return Ok(());
    }

    let r = crate::onchainos::wallet_contract_call(args.chain, cfg.npm, &collect_calldata, None, None, args.dry_run, args.confirm).await?;
    println!("  collect tx: {}", crate::onchainos::extract_tx_hash(&r));
    println!("\nLiquidity removed and tokens collected successfully!");

    Ok(())
}
