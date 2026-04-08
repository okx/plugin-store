/// `pancakeswap add-liquidity` — mint a new V3 LP position via NonfungiblePositionManager.

use anyhow::Result;

pub struct AddLiquidityArgs {
    pub token_a: String,
    pub token_b: String,
    pub fee: u32,
    pub amount_a: String,
    pub amount_b: String,
    pub tick_lower: i32,
    pub tick_upper: i32,
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

    // Validate tick spacing
    let spacing = crate::config::tick_spacing(args.fee)?;
    if args.tick_lower % spacing != 0 || args.tick_upper % spacing != 0 {
        anyhow::bail!(
            "Ticks must be multiples of tickSpacing ({}) for fee tier {}. Got tickLower={}, tickUpper={}",
            spacing, args.fee, args.tick_lower, args.tick_upper
        );
    }

    // Apply slippage to minimums using integer arithmetic (avoids f64 precision loss on large wei values)
    // slippage is in percent (e.g. 1.0 means 1%), convert to bps (100 bps)
    let slippage_bps = (args.slippage * 100.0) as u128;
    let amount0_min = amount0_desired.saturating_mul(10000 - slippage_bps) / 10000;
    let amount1_min = amount1_desired.saturating_mul(10000 - slippage_bps) / 10000;

    // Deadline: 20 minutes from now
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() + 1200)
        .unwrap_or(9_999_999_999);

    println!("Add Liquidity (chain {}):", args.chain);
    println!("  Token0 (token0 < token1): {} {}", amount_a_str, sym0);
    println!("  Token1:                   {} {}", amount_b_str, sym1);
    println!("  Fee tier:                 {}%", args.fee as f64 / 10000.0);
    println!("  Tick range:               {} to {}", args.tick_lower, args.tick_upper);
    println!("  NPM:                      {}", cfg.npm);

    // Fetch wallet address for use as recipient in mint
    let wallet_address = if args.dry_run {
        "0x0000000000000000000000000000000000000001".to_string()
    } else {
        crate::onchainos::get_wallet_address().await?
    };

    // Step 1: Approve token0 for NPM
    println!("\nStep 1: Approving {} for NonfungiblePositionManager...", sym0);
    let approve0_calldata = crate::calldata::encode_approve_max(cfg.npm)?;

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
    let approve1_calldata = crate::calldata::encode_approve_max(cfg.npm)?;

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
        args.tick_lower,
        args.tick_upper,
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
    println!("\nLP position minted successfully!");

    Ok(())
}
