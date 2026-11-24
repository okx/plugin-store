/// `pancakeswap-v3 swap` — exact-input token swap via SmartRouter.

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

    if from_addr == to_addr {
        anyhow::bail!("tokenIn and tokenOut must be different tokens.");
    }

    // Resolve token metadata
    let decimals_in = crate::rpc::get_decimals(&from_addr, cfg.rpc_url).await.unwrap_or(18);
    let decimals_out = crate::rpc::get_decimals(&to_addr, cfg.rpc_url).await.unwrap_or(18);
    let symbol_in = crate::rpc::get_symbol(&from_addr, cfg.rpc_url).await.unwrap_or_else(|_| args.from.clone());
    let symbol_out = crate::rpc::get_symbol(&to_addr, cfg.rpc_url).await.unwrap_or_else(|_| args.to.clone());

    let amount_in = crate::config::human_to_minimal(&args.amount, decimals_in)?;

    if amount_in == 0 {
        anyhow::bail!("Amount must be greater than 0.");
    }

    // Fetch wallet address early so we can check balance before RPC quote calls
    let wallet_addr = crate::onchainos::get_wallet_address().await
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());

    // Check sufficient balance before making expensive quote RPC calls.
    // Skip in dry-run: wallet may not be connected and the calldata preview is still useful.
    if !args.dry_run {
        let balance_in = crate::rpc::get_balance(&from_addr, &wallet_addr, cfg.rpc_url).await?;
        if balance_in < amount_in {
            let have = balance_in as f64 / 10f64.powi(decimals_in as i32);
            let need = amount_in as f64 / 10f64.powi(decimals_in as i32);
            anyhow::bail!(
                "Insufficient balance: have {:.6} {}, need {:.6} {}",
                have, symbol_in, need, symbol_in
            );
        }
    }

    // Get best quote across fee tiers, verifying pool has actual liquidity
    let fee_tiers = [100u32, 500, 2500, 10000];
    let mut best_out = 0u128;
    let mut best_fee = 500u32;

    for fee in fee_tiers {
        // Verify pool exists via factory (non-zero address = pool deployed)
        let pool_exists = crate::rpc::get_pool_address(
            cfg.factory, &from_addr, &to_addr, fee, cfg.rpc_url
        ).await.is_ok();
        if !pool_exists {
            continue;
        }

        match crate::rpc::quote_exact_input_single(
            cfg.quoter_v2,
            &from_addr,
            &to_addr,
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
            "No liquidity found for {}/{} on chain {}. Use `pancakeswap-v3 pools` to verify pools exist.",
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

    // Preview gate: without --confirm (or with --dry-run), show intent and stop.
    if args.dry_run || !args.confirm {
        let approve_calldata = crate::calldata::encode_approve(cfg.smart_router, amount_in)?;
        let swap_calldata = crate::calldata::encode_exact_input_single(
            &from_addr,
            &to_addr,
            best_fee,
            &wallet_addr,
            amount_in,
            amount_out_minimum,
            cfg.swap_with_deadline,
        )?;
        println!("\nPreview (no transactions broadcast — add --confirm to execute):");
        println!("  Step 1 approve {} {} to SmartRouter:", args.amount, symbol_in);
        println!("    onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, from_addr, approve_calldata);
        println!("  Step 2 swap {} {} → min {:.6} {} ({}% slippage):", args.amount, symbol_in, amount_out_min_human, symbol_out, args.slippage);
        println!("    onchainos wallet contract-call --chain {} --to {} --input-data {}", args.chain, cfg.smart_router, swap_calldata);
        return Ok(());
    }

    // Step 1: Approve SmartRouter to spend tokenIn (skip if allowance already sufficient)
    println!("\nStep 1: Approving SmartRouter to spend {}...", symbol_in);
    let approve_calldata = crate::calldata::encode_approve(cfg.smart_router, amount_in)?;

    // Check existing allowance to avoid unnecessary approve (prevents nonce conflicts)
    let allowance = crate::rpc::get_allowance(&from_addr, &wallet_addr, cfg.smart_router, cfg.rpc_url)
        .await
        .unwrap_or_else(|e| {
            eprintln!("  [warn] allowance check failed ({}), proceeding with approve.", e);
            0
        });
    if allowance >= amount_in {
        println!("  Allowance already sufficient ({}), skipping approve.", allowance);
    } else {
        let approve_result = crate::onchainos::wallet_contract_call(
            args.chain,
            &from_addr,
            &approve_calldata,
            None,
            None,
            args.dry_run,
            args.confirm,
        ).await?;
        let approve_tx = crate::onchainos::extract_tx_hash(&approve_result);
        println!("  Approve tx: {}", approve_tx);
        // Wait for approve to be confirmed on-chain before submitting swap.
        // 3s was too short for Ethereum (~12s blocks) — STF revert if swap lands first.
        crate::onchainos::wait_and_check_receipt(approve_tx, cfg.rpc_url).await?;
    }

    // Step 2: Execute swap via SmartRouter.exactInputSingle
    let recipient_placeholder = wallet_addr;

    println!("\nStep 2: Executing swap via SmartRouter.exactInputSingle...");
    let swap_calldata = crate::calldata::encode_exact_input_single(
        &from_addr,
        &to_addr,
        best_fee,
        &recipient_placeholder,
        amount_in,
        amount_out_minimum,
        cfg.swap_with_deadline,
    )?;

    let swap_result = crate::onchainos::wallet_contract_call(
        args.chain,
        cfg.smart_router,
        &swap_calldata,
        None,
        None,
        args.dry_run,
        args.confirm,
    ).await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&swap_result);
    println!("  Swap tx: {}", tx_hash);
    println!("\nSwap submitted successfully!");
    println!("  Swapped {} {} -> ~{:.6} {}", args.amount, symbol_in, amount_out_human, symbol_out);

    Ok(())
}
