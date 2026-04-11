/// `pancakeswap add-liquidity` — mint a new V3 LP position via NonfungiblePositionManager.

use anyhow::Result;

pub struct AddLiquidityArgs {
    pub token_a: String,
    pub token_b: String,
    pub fee: u32,
    pub amount_a: String,
    pub amount_b: String,
    pub tick_lower: Option<i32>,
    pub tick_upper: Option<i32>,
    pub slippage: f64,
    pub chain: u64,
    pub dry_run: bool,
    pub confirm: bool,
}

pub async fn run(args: AddLiquidityArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    // Resolve token symbols to addresses first
    let addr_a = crate::config::resolve_token_address(&args.token_a, args.chain)?;
    let addr_b = crate::config::resolve_token_address(&args.token_b, args.chain)?;

    // Sort tokens: token0 < token1 numerically (required by NonfungiblePositionManager)
    let (token0, token1) = crate::calldata::sort_tokens(&addr_a, &addr_b)?;
    let (amount_a_str, amount_b_str) = if token0 == addr_a.as_str() {
        (args.amount_a.as_str(), args.amount_b.as_str())
    } else {
        (args.amount_b.as_str(), args.amount_a.as_str())
    };

    let decimals0 = crate::rpc::get_decimals(token0, cfg.rpc_url).await.unwrap_or(18);
    let decimals1 = crate::rpc::get_decimals(token1, cfg.rpc_url).await.unwrap_or(18);
    let sym0 = crate::rpc::get_symbol(token0, cfg.rpc_url).await.unwrap_or_else(|_| token0.to_string());
    let sym1 = crate::rpc::get_symbol(token1, cfg.rpc_url).await.unwrap_or_else(|_| token1.to_string());

    let amount0_desired = crate::config::human_to_minimal(amount_a_str, decimals0)?;
    let amount1_desired = crate::config::human_to_minimal(amount_b_str, decimals1)?;

    if amount0_desired == 0 && amount1_desired == 0 {
        anyhow::bail!("Both amounts are zero — provide at least one non-zero amount.");
    }

    let spacing = crate::config::tick_spacing(args.fee)?;

    // Resolve tick range + fetch pool slot0 (needed for both auto-tick and slippage math)
    let pool = crate::rpc::get_pool_address(cfg.factory, token0, token1, args.fee, cfg.rpc_url).await
        .map_err(|e| anyhow::anyhow!("Could not find pool (fee {}, chain {}): {}. Try specifying --tick-lower and --tick-upper manually.", args.fee, args.chain, e))?;
    let (sqrt_price_x96, current_tick) = crate::rpc::get_slot0(&pool, cfg.rpc_url).await?;

    let (tick_lower, tick_upper) = match (args.tick_lower, args.tick_upper) {
        (Some(tl), Some(tu)) => {
            if tl % spacing != 0 || tu % spacing != 0 {
                anyhow::bail!(
                    "Ticks must be multiples of tickSpacing ({}) for fee tier {}. Got tickLower={}, tickUpper={}",
                    spacing, args.fee, tl, tu
                );
            }
            if tl >= tu {
                anyhow::bail!("tickLower ({}) must be less than tickUpper ({})", tl, tu);
            }
            (tl, tu)
        }
        (None, None) => {
            // Auto-compute: ±10% price range ≈ ±1000 ticks, aligned to tickSpacing.
            let range = 1000i32.max(spacing * 20);
            // Euclidean division so negative ticks round toward −∞ (correct alignment)
            let tl = (current_tick - range).div_euclid(spacing) * spacing;
            let tu = (current_tick + range).div_euclid(spacing) * spacing;
            println!("Auto tick range: {} to {} (current tick: {}, ±{} ticks)", tl, tu, current_tick, range);
            (tl, tu)
        }
        _ => anyhow::bail!("Provide both --tick-lower and --tick-upper, or omit both for auto ±10% range."),
    };

    // Compute actual deposit amounts using V3 math, then apply slippage to those.
    // V3 deposits the optimal ratio for current price — applying slippage to the
    // desired amounts produces incorrect (too-tight) minimums and causes reverts.
    let (actual0, actual1) = crate::rpc::amounts_for_add_liquidity(
        sqrt_price_x96, tick_lower, tick_upper, current_tick,
        amount0_desired, amount1_desired,
    );
    let slippage_bps = (args.slippage * 100.0) as u128;
    let amount0_min = actual0.saturating_mul(10000 - slippage_bps) / 10000;
    let amount1_min = actual1.saturating_mul(10000 - slippage_bps) / 10000;
    println!("Expected deposit: {} {} / {} {} → min: {} / {} ({}% slippage)",
        actual0, sym0, actual1, sym1, amount0_min, amount1_min, args.slippage);

    // Deadline: 20 minutes from now
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() + 1200)
        .unwrap_or(9_999_999_999);

    // Fetch wallet address early — needed for balance check and as mint recipient
    let wallet_address = if args.dry_run {
        "0x0000000000000000000000000000000000000001".to_string()
    } else {
        crate::onchainos::get_wallet_address().await?
    };

    // Bug 1 fix: pre-flight balance check — bail before wasting gas on approve
    if !args.dry_run {
        let bal0 = crate::rpc::get_balance(token0, &wallet_address, cfg.rpc_url).await?;
        let bal1 = crate::rpc::get_balance(token1, &wallet_address, cfg.rpc_url).await?;
        if bal0 < amount0_desired {
            anyhow::bail!(
                "Insufficient {} balance: wallet has {} but {} required (minimal units). Deposit more {} before adding liquidity.",
                sym0, bal0, amount0_desired, sym0
            );
        }
        if bal1 < amount1_desired {
            anyhow::bail!(
                "Insufficient {} balance: wallet has {} but {} required (minimal units). Deposit more {} before adding liquidity.",
                sym1, bal1, amount1_desired, sym1
            );
        }
        println!("Balance check OK: {} {} available, {} {} available", bal0, sym0, bal1, sym1);
    }

    println!("Add Liquidity (chain {}):", args.chain);
    println!("  Token0 (token0 < token1): {} {}", amount_a_str, sym0);
    println!("  Token1:                   {} {}", amount_b_str, sym1);
    println!("  Fee tier:                 {}%", args.fee as f64 / 10000.0);
    println!("  Tick range:               {} to {}", tick_lower, tick_upper);
    println!("  NPM:                      {}", cfg.npm);

    // Step 1: Approve token0 for NPM
    println!("\nStep 1: Approving {} for NonfungiblePositionManager...", sym0);
    eprintln!("WARNING: Approving {} {} to {} -- approving exact amount only. Use --dry-run to preview.", amount0_desired, sym0, cfg.npm);
    let approve0_calldata = crate::calldata::encode_approve(cfg.npm, amount0_desired)?;

    if args.dry_run {
        println!("  [dry-run] onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, token0, approve0_calldata);
    } else {
        let r = crate::onchainos::wallet_contract_call(args.chain, token0, &approve0_calldata, None, None, args.dry_run, args.confirm).await?;
        println!("  Approve tx: {}", crate::onchainos::extract_tx_hash(&r));
        // Wait for nonce to settle before next sequential transaction
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Step 2: Approve token1 for NPM
    println!("\nStep 2: Approving {} for NonfungiblePositionManager...", sym1);
    eprintln!("WARNING: Approving {} {} to {} -- approving exact amount only. Use --dry-run to preview.", amount1_desired, sym1, cfg.npm);
    let approve1_calldata = crate::calldata::encode_approve(cfg.npm, amount1_desired)?;

    if args.dry_run {
        println!("  [dry-run] onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, token1, approve1_calldata);
    } else {
        let r = crate::onchainos::wallet_contract_call(args.chain, token1, &approve1_calldata, None, None, args.dry_run, args.confirm).await?;
        println!("  Approve tx: {}", crate::onchainos::extract_tx_hash(&r));
        // Wait for nonce to settle before mint
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    // Step 3: Mint position
    println!("\nStep 3: Minting LP position via NonfungiblePositionManager.mint...");
    println!("  Recipient:                {}", wallet_address);
    let mint_calldata = crate::calldata::encode_mint(
        token0,
        token1,
        args.fee,
        tick_lower,
        tick_upper,
        amount0_desired,
        amount1_desired,
        amount0_min,
        amount1_min,
        &wallet_address,
        deadline,
    )?;

    if args.dry_run {
        println!("  [dry-run] onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, cfg.npm, mint_calldata);
        println!("\nDry-run complete. No transactions submitted.");
        return Ok(());
    }

    let r = crate::onchainos::wallet_contract_call(args.chain, cfg.npm, &mint_calldata, None, None, args.dry_run, args.confirm).await?;
    let tx_hash = crate::onchainos::extract_tx_hash(&r);
    println!("  Mint tx: {}", tx_hash);
    println!("  Waiting for on-chain confirmation...");
    crate::onchainos::wait_and_check_receipt(tx_hash, cfg.rpc_url).await?;
    println!("\nLP position minted successfully!");

    Ok(())
}
