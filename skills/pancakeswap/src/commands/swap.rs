/// `pancakeswap swap` — exact-input token swap via SmartRouter.

use anyhow::Result;

pub struct SwapArgs {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub slippage: f64,
    pub chain: u64,
    pub dry_run: bool,
    pub confirm: bool,
}

pub async fn run(args: SwapArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    // Resolve token symbols to addresses
    let from_addr = crate::config::resolve_token_address(&args.from, args.chain)?;
    let to_addr = crate::config::resolve_token_address(&args.to, args.chain)?;

    let is_native_in  = crate::config::is_native_token(&from_addr);
    let is_native_out = crate::config::is_native_token(&to_addr);

    // For SmartRouter/QuoterV2 calls, substitute native sentinel with wrapped ERC-20 address
    let erc20_from = if is_native_in  { crate::config::wrapped_native(args.chain).to_string() } else { from_addr.clone() };
    let erc20_to   = if is_native_out { crate::config::wrapped_native(args.chain).to_string() } else { to_addr.clone() };

    // Resolve token metadata — native tokens are always 18 decimals
    let decimals_in  = if is_native_in  { 18u8 } else { crate::rpc::get_decimals(&from_addr, cfg.rpc_url).await.unwrap_or(18) };
    let decimals_out = if is_native_out { 18u8 } else { crate::rpc::get_decimals(&to_addr, cfg.rpc_url).await.unwrap_or(18) };

    // Use input symbol for native tokens (avoids showing "WBNB" when user typed "BNB")
    let symbol_in  = if is_native_in  { args.from.to_uppercase() }
        else { crate::rpc::get_symbol(&from_addr, cfg.rpc_url).await.unwrap_or_else(|_| args.from.clone()) };
    let symbol_out = if is_native_out { args.to.to_uppercase() }
        else { crate::rpc::get_symbol(&to_addr, cfg.rpc_url).await.unwrap_or_else(|_| args.to.clone()) };

    let amount_in = crate::config::human_to_minimal(&args.amount, decimals_in)?;

    // Get best quote across fee tiers, verifying pool has actual liquidity
    let fee_tiers = [100u32, 500, 2500, 10000];
    let mut best_out = 0u128;
    let mut best_fee = 500u32;

    for fee in fee_tiers {
        // Verify pool exists via factory (non-zero address = pool deployed)
        let pool_exists = crate::rpc::get_pool_address(
            cfg.factory, &erc20_from, &erc20_to, fee, cfg.rpc_url
        ).await.is_ok();
        if !pool_exists {
            continue;
        }

        match crate::rpc::quote_exact_input_single(
            cfg.quoter_v2,
            &erc20_from,
            &erc20_to,
            amount_in,
            fee,
            cfg.rpc_url,
        ).await {
            Ok(out) if out > best_out => {
                best_out = out;
                best_fee = fee;
            }
            _ => {}
        }
    }

    if best_out == 0 {
        anyhow::bail!(
            "No liquidity found for {}/{} on chain {}. Use `pancakeswap pools` to verify pools exist.",
            symbol_in, symbol_out, args.chain
        );
    }

    // Apply slippage tolerance using integer arithmetic (avoids f64 precision loss on large wei values)
    // slippage is in percent (e.g. 0.5 means 0.5%), convert to bps (50 bps)
    let slippage_bps = (args.slippage * 100.0) as u128;
    let amount_out_minimum = best_out.saturating_mul(10000 - slippage_bps) / 10000;

    let amount_out_human = best_out as f64 / 10f64.powi(decimals_out as i32);
    let amount_out_min_human = amount_out_minimum as f64 / 10f64.powi(decimals_out as i32);

    println!("Swap (chain {}):", args.chain);
    println!("  From:             {} {}", args.amount, symbol_in);
    println!("  Expected output:  {:.6} {}", amount_out_human, symbol_out);
    println!("  Minimum output:   {:.6} {} ({}% slippage)", amount_out_min_human, symbol_out, args.slippage);
    println!("  Fee tier:         {}%", best_fee as f64 / 10000.0);
    println!("  SmartRouter:      {}", cfg.smart_router);

    // Fetch actual wallet address (needed for approve check and swap recipient)
    let wallet_addr = crate::onchainos::get_wallet_address().await
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());

    // Preview gate: without --confirm (or with --dry-run), show intent and stop.
    if args.dry_run || !args.confirm {
        let swap_calldata = crate::calldata::encode_exact_input_single(
            &erc20_from,
            &erc20_to,
            best_fee,
            &wallet_addr,
            amount_in,
            amount_out_minimum,
        )?;
        println!("\nPreview (no transactions broadcast — add --confirm to execute):");
        if !is_native_in {
            let approve_calldata = crate::calldata::encode_approve(cfg.smart_router, amount_in)?;
            println!("  Step 1 approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, erc20_from, approve_calldata);
            println!("  Step 2 swap:    onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, cfg.smart_router, swap_calldata);
        } else {
            println!("  Step 1 swap:    onchainos wallet contract-call --chain {} --to {} --input-data {} --amt {}", args.chain, cfg.smart_router, swap_calldata, amount_in);
            println!("  (Native {} sent as msg.value — no approve needed)", symbol_in);
        }
        return Ok(());
    }

    let swap_calldata = crate::calldata::encode_exact_input_single(
        &erc20_from,
        &erc20_to,
        best_fee,
        &wallet_addr,
        amount_in,
        amount_out_minimum,
    )?;

    // Step 1: Approve SmartRouter to spend tokenIn — skipped for native BNB/ETH
    if is_native_in {
        println!("\nNative {} detected — skipping approve (msg.value sent with swap tx)", symbol_in);
    } else {
        println!("\nStep 1: Approving SmartRouter to spend {}...", symbol_in);
        let approve_calldata = crate::calldata::encode_approve(cfg.smart_router, amount_in)?;

        // Check existing allowance to avoid unnecessary approve (prevents nonce conflicts)
        let allowance = crate::rpc::get_allowance(&erc20_from, &wallet_addr, cfg.smart_router, cfg.rpc_url)
            .await.unwrap_or(0);
        if allowance >= amount_in {
            println!("  Allowance already sufficient ({}), skipping approve.", allowance);
        } else {
            let approve_result = crate::onchainos::wallet_contract_call(
                args.chain,
                &erc20_from,
                &approve_calldata,
                None,
                None,
                args.dry_run,
                args.confirm,
            ).await?;
            let approve_tx = crate::onchainos::extract_tx_hash(&approve_result);
            println!("  Approve tx: {}", approve_tx);
            // Wait for approve to be processed before submitting swap
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    // Step 2: Execute swap via SmartRouter.exactInputSingle
    // For native input: send amount_in as msg.value — SmartRouter wraps it automatically.
    // For native output: user receives WBNB/WETH (they can unwrap separately).
    let native_value: Option<u128> = if is_native_in { Some(amount_in) } else { None };

    let step = if is_native_in { 1 } else { 2 };
    println!("\nStep {}: Executing swap via SmartRouter.exactInputSingle...", step);

    let swap_result = crate::onchainos::wallet_contract_call(
        args.chain,
        cfg.smart_router,
        &swap_calldata,
        None,
        native_value,
        args.dry_run,
        args.confirm,
    ).await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&swap_result);
    println!("  Swap tx: {}", tx_hash);
    println!("\nSwap submitted successfully!");
    println!("  Swapped {} {} -> ~{:.6} {}", args.amount, symbol_in, amount_out_human, symbol_out);
    if is_native_out {
        println!("  Note: received as W{} (wrapped). Unwrap via the WBNB/WETH contract if you need native {}.", symbol_out, symbol_out);
    }

    Ok(())
}
