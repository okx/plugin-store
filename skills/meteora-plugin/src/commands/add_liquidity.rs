use clap::Args;
use reqwest::Client;
use serde_json::json;
use solana_pubkey::Pubkey;
use std::str::FromStr;

use crate::meteora_ix;
use crate::onchainos;
use crate::solana_rpc;

#[derive(Args, Debug)]
pub struct AddLiquidityArgs {
    /// Meteora DLMM pool (LbPair) address
    #[arg(long)]
    pub pool: String,

    /// Amount of token X to deposit (human-readable, e.g. "0.01")
    #[arg(long, default_value = "0")]
    pub amount_x: f64,

    /// Amount of token Y to deposit (human-readable, e.g. "1.5")
    #[arg(long, default_value = "0")]
    pub amount_y: f64,

    /// Half-range in bins around the active bin (total = 2*bin_range+1 bins). Default: 10
    #[arg(long, default_value = "10")]
    pub bin_range: i32,

    /// Wallet address (Solana pubkey). If omitted, uses the currently logged-in onchainos wallet.
    #[arg(long)]
    pub wallet: Option<String>,
}

/// Bins of tolerance for Y-only deposits: liq_upper = active_id - 1 - Y_ONLY_SLIPPAGE
/// so the Y range stays below active_id even if price drifts down by up to 5 bins between
/// reading the pool state and executing the transaction.
const Y_ONLY_SLIPPAGE: i32 = 5;

pub async fn execute(args: &AddLiquidityArgs, dry_run: bool) -> anyhow::Result<()> {
    let client = Client::new();

    // ── 1. Resolve wallet ────────────────────────────────────────────────────
    let wallet_str = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!("Cannot resolve wallet. Pass --wallet or log in via onchainos.\nError: {e}")
        })?
    };

    let wallet =
        Pubkey::from_str(&wallet_str).map_err(|e| anyhow::anyhow!("Invalid wallet: {e}"))?;
    let lb_pair =
        Pubkey::from_str(&args.pool).map_err(|e| anyhow::anyhow!("Invalid pool: {e}"))?;

    // ── 2. Fetch & parse LbPair account ─────────────────────────────────────
    let pool_data = solana_rpc::get_account_data(&client, &args.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch pool {}: {e}", args.pool))?;
    let pool = solana_rpc::parse_lb_pair(&pool_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse LbPair: {e}"))?;

    let token_x_mint = Pubkey::from(pool.token_x_mint);
    let token_y_mint = Pubkey::from(pool.token_y_mint);
    let reserve_x = Pubkey::from(pool.reserve_x);
    let reserve_y = Pubkey::from(pool.reserve_y);

    // Native SOL mint — used to detect when WSOL wrap is needed
    const WSOL_MINT: Pubkey =
        solana_pubkey::pubkey!("So11111111111111111111111111111111111111112");

    // ── 3. Fetch token decimals ──────────────────────────────────────────────
    let mint_x_str = token_x_mint.to_string();
    let mint_y_str = token_y_mint.to_string();
    let (mint_x_data, mint_y_data) = tokio::try_join!(
        solana_rpc::get_account_data(&client, &mint_x_str),
        solana_rpc::get_account_data(&client, &mint_y_str),
    )?;
    let decimals_x = solana_rpc::parse_mint_decimals(&mint_x_data);
    let decimals_y = solana_rpc::parse_mint_decimals(&mint_y_data);

    // ── 4. Convert amounts to raw u64 ────────────────────────────────────────
    let amount_x_raw = (args.amount_x * 10f64.powi(decimals_x as i32)).round() as u64;
    let amount_y_raw = (args.amount_y * 10f64.powi(decimals_y as i32)).round() as u64;

    anyhow::ensure!(
        amount_x_raw > 0 || amount_y_raw > 0,
        "Both --amount-x and --amount-y are 0. Specify at least one non-zero amount."
    );

    // ── 4.5 Balance pre-flight check ─────────────────────────────────────────
    {
        let min_sol = 0.01_f64; // gas only
        let sol_balance = onchainos::get_sol_balance(&wallet_str);

        // Token X check (skip if WSOL — SOL is checked separately)
        if amount_x_raw > 0 && token_x_mint != WSOL_MINT {
            let bal_x = onchainos::get_spl_token_balance(&mint_x_str);
            if bal_x < args.amount_x {
                let output = json!({
                    "ok": false,
                    "error": format!(
                        "Insufficient token X balance. Required: {:.6}, available: {:.6}. Please top up token X ({}).",
                        args.amount_x, bal_x, mint_x_str
                    ),
                    "required": args.amount_x,
                    "available": bal_x,
                    "token": mint_x_str,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
                return Ok(());
            }
        }

        // SOL check: covers gas + WSOL wrap if depositing SOL as X
        let sol_needed = min_sol + if token_x_mint == WSOL_MINT { args.amount_x } else { 0.0 };
        if sol_balance < sol_needed {
            let output = json!({
                "ok": false,
                "error": format!(
                    "Insufficient SOL balance. Required: ~{:.4} SOL (deposit: {} + gas: ~0.01), available: {:.6} SOL.",
                    sol_needed,
                    if token_x_mint == WSOL_MINT { format!("{}", args.amount_x) } else { "0".to_string() },
                    sol_balance
                ),
                "required_sol": sol_needed,
                "available_sol": sol_balance,
                "wallet": wallet_str,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        // Token Y check
        if amount_y_raw > 0 {
            let bal_y = if token_y_mint == WSOL_MINT {
                sol_balance - sol_needed
            } else {
                onchainos::get_spl_token_balance(&mint_y_str)
            };
            if bal_y < args.amount_y {
                let output = json!({
                    "ok": false,
                    "error": format!(
                        "Insufficient token Y balance. Required: {:.6}, available: {:.6}. Please top up token Y ({}).",
                        args.amount_y, bal_y, mint_y_str
                    ),
                    "required": args.amount_y,
                    "available": bal_y,
                    "token": mint_y_str,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
                return Ok(());
            }
        }
    }

    // ── 5. Compute position range ────────────────────────────────────────────
    // Meteora rejects creating a new position whose bin range overlaps with an
    // existing position for the same (owner, lb_pair). Strategy:
    //   1. Scan on-chain positions for this wallet + pool.
    //   2. If one already spans the active_id, reuse it (deposit into it).
    //   3. Otherwise pick a non-overlapping range.
    const MAX_BIN_PER_POSITION: i32 = 70;

    let y_only = amount_x_raw == 0 && amount_y_raw > 0;

    // Compute desired liq range based on deposit type
    let (liq_lower, liq_upper) = match (amount_x_raw > 0, amount_y_raw > 0) {
        (true, false) => {
            // X-only: bins above active_id
            (pool.active_id, pool.active_id + args.bin_range)
        }
        (false, true) => {
            // Y-only: bins below active_id, with slippage guard so all bins stay Y-side
            // even if active_id drifts down by Y_ONLY_SLIPPAGE bins
            let upper = pool.active_id - 1 - Y_ONLY_SLIPPAGE;
            (upper - args.bin_range + 1, upper)
        }
        _ => {
            // Two-sided
            (pool.active_id - args.bin_range, pool.active_id + args.bin_range)
        }
    };

    // Scan for existing positions: find one that spans active_id (can deposit into it)
    // or determine a non-overlapping range for a new position.
    let existing_positions = solana_rpc::get_dlmm_positions_by_owner(
        &client,
        &meteora_ix::DLMM_PROGRAM.to_string(),
        &wallet_str,
        Some(&args.pool),
    ).await.unwrap_or_default();

    // Try to find an existing position that spans the active_id (reusable)
    let spanning_pos = existing_positions.iter().find(|p| {
        p.lower_bin_id <= pool.active_id && p.upper_bin_id >= pool.active_id
    });

    let (pos_lower, width, pos_upper, position_exists) = if let Some(sp) = spanning_pos {
        let w = sp.upper_bin_id - sp.lower_bin_id + 1;
        (sp.lower_bin_id, w, sp.upper_bin_id, true)
    } else {
        // No spanning position — create a new one.
        // Find a non-overlapping center. Start from standard center and adjust if needed.
        let mut center = pool.active_id;

        // Check if standard center would overlap existing positions
        for ep in &existing_positions {
            let trial_lower = center - MAX_BIN_PER_POSITION / 2;
            let trial_upper = trial_lower + MAX_BIN_PER_POSITION - 1;
            if trial_lower <= ep.upper_bin_id && trial_upper >= ep.lower_bin_id {
                // Overlap: shift center past this position
                center = ep.upper_bin_id + MAX_BIN_PER_POSITION / 2 + 1;
            }
        }

        let pl = center - MAX_BIN_PER_POSITION / 2;
        let w = MAX_BIN_PER_POSITION;
        let pu = pl + w - 1;
        (pl, w, pu, false)
    };

    // Clamp liq range to fit within position boundaries.
    let (liq_lower, liq_upper) = if y_only {
        let cl = liq_lower.max(pos_lower);
        if cl != liq_lower {
            eprintln!(
                "[warn] Y-only bin_range {} clamped: position [{}, {}] only has {} bins below active_id {}; using {} bins",
                args.bin_range, pos_lower, pos_upper, pool.active_id - 1 - pos_lower + 1, pool.active_id,
                liq_upper - cl + 1
            );
        }
        (cl, liq_upper)
    } else if amount_x_raw > 0 && amount_y_raw == 0 {
        let cu = liq_upper.min(pos_upper);
        if cu != liq_upper {
            eprintln!(
                "[warn] X-only bin_range clamped to position upper; using {} bins",
                cu - liq_lower + 1
            );
        }
        (liq_lower, cu)
    } else {
        (liq_lower.max(pos_lower), liq_upper.min(pos_upper))
    };

    anyhow::ensure!(
        liq_lower <= liq_upper,
        "No bins available for deposit at active_id={}; position [{}, {}] is fully outside the liq zone. \
         Wait for price to move into position range and retry.",
        pool.active_id, pos_lower, pos_upper
    );

    // ── 6. Derive PDAs ───────────────────────────────────────────────────────
    let position = meteora_ix::position_pda(&lb_pair, &wallet, pos_lower, width);
    // DLMM requires bin_array_lower.index < bin_array_upper.index (program cannot
    // borrow the same account twice).
    //
    // The Meteora program validates that the passed bin arrays span the POSITION's
    // full bin range (pos_lower..pos_upper), not just the deposit range
    // (liq_lower..liq_upper). Derive indices from pos_lower / pos_upper so the
    // arrays always cover the entire position even for narrow X-only / Y-only deposits.
    //
    // Edge case: if pos_lower and pos_upper fall in the same bin array (happens when
    // pos_lower lands exactly at an array boundary so all 70 bins stay in one array),
    // use the adjacent array on the appropriate side as the structural second account.
    let pos_lower_arr = meteora_ix::bin_array_index(pos_lower);
    let pos_upper_arr = meteora_ix::bin_array_index(pos_upper);
    let (lower_idx, upper_idx) = if pos_lower_arr == pos_upper_arr {
        if amount_x_raw > 0 && amount_y_raw == 0 {
            // X-only (bins go up): placeholder is the next-higher array
            (pos_lower_arr, pos_lower_arr + 1)
        } else {
            // Y-only or two-sided (bins go down / both sides): placeholder is lower
            (pos_lower_arr - 1, pos_lower_arr)
        }
    } else {
        (pos_lower_arr, pos_upper_arr)
    };
    let (effective_liq_lower, effective_liq_upper) = (liq_lower, liq_upper);
    let bin_array_lower = meteora_ix::bin_array_pda(&lb_pair, lower_idx);
    let bin_array_upper = meteora_ix::bin_array_pda(&lb_pair, upper_idx);
    // Precompute ATAs to use as hints for find_token_account
    let ata_x = meteora_ix::get_ata(&wallet, &token_x_mint);
    let ata_y = meteora_ix::get_ata(&wallet, &token_y_mint);

    // ── 7. Resolve token accounts and check position existence ──────────────
    let ata_x_str = ata_x.to_string();
    let ata_y_str = ata_y.to_string();
    let pos_str = position.to_string();
    let mint_x_str2 = token_x_mint.to_string();
    let mint_y_str2 = token_y_mint.to_string();
    let ((token_x_acct, ata_x_exists), (token_y_acct, ata_y_exists), position_exists_onchain) =
        tokio::try_join!(
            solana_rpc::find_token_account(&client, &wallet_str, &mint_x_str2, &ata_x_str),
            solana_rpc::find_token_account(&client, &wallet_str, &mint_y_str2, &ata_y_str),
            solana_rpc::account_exists(&client, &pos_str),
        )?;
    // Use on-chain account_exists as the authoritative source.
    // get_dlmm_positions_by_owner (getProgramAccounts) can return stale data — e.g. a
    // recently-closed position may still appear in the index for a few seconds after
    // close_position_if_empty confirms. If we trust stale data we skip ix_initialize_position_pda,
    // then the DLMM instruction fails with "account owned by a different program" because the
    // closed account reverts to System Program ownership.
    let position_exists = position_exists_onchain;
    let user_token_x: Pubkey = token_x_acct.parse()?;
    let user_token_y: Pubkey = token_y_acct.parse()?;

    // ── 8. Dry-run output ────────────────────────────────────────────────────
    if dry_run {
        let output = json!({
            "ok": true,
            "dry_run": true,
            "message": "Dry run: preview only, no transaction submitted.",
            "pool": args.pool,
            "wallet": wallet_str,
            "token_x_mint": token_x_mint.to_string(),
            "token_y_mint": token_y_mint.to_string(),
            "token_x_decimals": decimals_x,
            "token_y_decimals": decimals_y,
            "active_id": pool.active_id,
            "bin_step": pool.bin_step,
            "position_lower_bin_id": pos_lower,
            "position_upper_bin_id": pos_upper,
            "position_width": width,
            "liq_lower_bin_id": effective_liq_lower,
            "liq_upper_bin_id": effective_liq_upper,
            "amount_x": args.amount_x,
            "amount_x_raw": amount_x_raw,
            "amount_y": args.amount_y,
            "amount_y_raw": amount_y_raw,
            "position_pda": position.to_string(),
            "position_exists": position_exists,
            "will_initialize_position": !position_exists,
            "bin_array_lower_idx": lower_idx,
            "bin_array_upper_idx": upper_idx,
            "bin_array_lower_pda": bin_array_lower.to_string(),
            "bin_array_upper_pda": bin_array_upper.to_string(),
            "user_token_x_account": user_token_x.to_string(),
            "user_token_x_exists": ata_x_exists,
            "user_token_y_account": user_token_y.to_string(),
            "user_token_y_exists": ata_y_exists,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // ── 10-13. Two-phase submission ──────────────────────────────────────────
    // onchainos runs Solana simulateTransaction before broadcast. Simulation
    // fails with ProgramAccountNotFound when a tx both creates accounts (ATAs,
    // position PDA) and reads them in the same transaction — the new accounts
    // don't exist at simulation time.
    //
    // Fix: split into two transactions when setup is needed:
    //   Tx 1 (setup): create ATAs + WSOL wrap + init bin arrays + init position
    //   Tx 2 (liquidity): add_liquidity_by_strategy only
    //
    // When all accounts already exist (second deposit into same position), a
    // single transaction is used instead.
    let bin_arr_lower_str = bin_array_lower.to_string();
    let bin_arr_upper_str = bin_array_upper.to_string();

    let ba_lower_exists = solana_rpc::account_exists(&client, &bin_arr_lower_str).await?;
    let ba_upper_exists = solana_rpc::account_exists(&client, &bin_arr_upper_str).await?;

    let needs_setup = !ata_x_exists || !ata_y_exists || !position_exists
        || !ba_lower_exists || (lower_idx != upper_idx && !ba_upper_exists);

    let mut setup_tx_hash = String::new();

    if needs_setup {
        eprintln!("[setup] Creating missing accounts before adding liquidity...");
        let blockhash = solana_rpc::get_latest_blockhash(&client).await?;
        let mut setup_ixs = vec![meteora_ix::ix_set_compute_unit_limit(400_000)];

        if !ata_x_exists {
            setup_ixs.push(meteora_ix::ix_create_ata_idempotent(
                &wallet, &user_token_x, &wallet, &token_x_mint,
            ));
        }
        if !ata_y_exists {
            setup_ixs.push(meteora_ix::ix_create_ata_idempotent(
                &wallet, &user_token_y, &wallet, &token_y_mint,
            ));
        }
        if !ba_lower_exists {
            setup_ixs.push(meteora_ix::ix_initialize_bin_array(
                &lb_pair, &bin_array_lower, &wallet, lower_idx,
            ));
        }
        if lower_idx != upper_idx && !ba_upper_exists {
            setup_ixs.push(meteora_ix::ix_initialize_bin_array(
                &lb_pair, &bin_array_upper, &wallet, upper_idx,
            ));
        }
        if !position_exists {
            setup_ixs.push(meteora_ix::ix_initialize_position_pda(
                &wallet, &lb_pair, &position, pos_lower, width,
            ));
        }

        let setup_b58 = meteora_ix::build_tx_b58(&setup_ixs, &wallet, blockhash)?;
        eprintln!("[setup] Submitting setup tx ({} instructions)...", setup_ixs.len());
        let setup_result = onchainos::contract_call_solana(&setup_b58, &meteora_ix::DLMM_PROGRAM.to_string())?;
        let setup_ok = setup_result["ok"].as_bool().unwrap_or(false)
            || setup_result["data"]["ok"].as_bool().unwrap_or(false);
        setup_tx_hash = setup_result["data"]["txHash"]
            .as_str()
            .or_else(|| setup_result["txHash"].as_str())
            .unwrap_or("")
            .to_string();

        if !setup_ok {
            let err = setup_result.get("error")
                .or_else(|| setup_result["data"].get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!("Setup transaction failed: {err}\nsetup_result: {setup_result}");
        }
        eprintln!("[setup] Setup tx submitted: {setup_tx_hash}");
        eprintln!("[setup] Waiting 8 s for setup tx to confirm on-chain...");
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }

    // ── Tx 2 (or single tx): add_liquidity instruction ───────────────────────
    //
    // For one-sided deposits (X-only or Y-only), use addLiquidityByStrategyOneSide
    // which accepts a single token/reserve account and validates the range on just
    // one side of the active bin. addLiquidityByStrategy (two-sided) will silently
    // deposit 0 when one amount is zero (SpotBalanced) or reject the range
    // (SpotImBalanced), so it is only used when both amounts are non-zero.
    // ── WSOL wrapping (always, not just on first deposit) ────────────────────
    // Wrapping is placed in the liquidity TX (not setup TX) so it fires on EVERY
    // deposit — including repeat deposits where needs_setup=false and all accounts
    // already exist. The wSOL ATA is guaranteed to exist at this point:
    //   - first deposit:  created in setup TX, confirmed after the 8s wait
    //   - repeat deposit: pre-existing on-chain
    let blockhash = solana_rpc::get_latest_blockhash(&client).await?;
    let mut liq_ixs = vec![meteora_ix::ix_set_compute_unit_limit(400_000)];
    if token_x_mint == WSOL_MINT && amount_x_raw > 0 {
        liq_ixs.push(meteora_ix::ix_sol_transfer(&wallet, &user_token_x, amount_x_raw));
        liq_ixs.push(meteora_ix::ix_sync_native(&user_token_x));
    }
    if token_y_mint == WSOL_MINT && amount_y_raw > 0 {
        liq_ixs.push(meteora_ix::ix_sol_transfer(&wallet, &user_token_y, amount_y_raw));
        liq_ixs.push(meteora_ix::ix_sync_native(&user_token_y));
    }

    let liquidity_ix = if amount_x_raw > 0 && amount_y_raw == 0 {
        meteora_ix::ix_add_liquidity_by_strategy_one_side(
            &position, &lb_pair,
            &user_token_x, &reserve_x, &token_x_mint,
            &bin_array_lower, &bin_array_upper, &wallet,
            amount_x_raw,
            pool.active_id,
            100,   // max_active_bin_slippage — 100 bins tolerance for active_id drift
            effective_liq_lower,
            effective_liq_upper,
        )
    } else if amount_y_raw > 0 && amount_x_raw == 0 {
        meteora_ix::ix_add_liquidity_by_strategy_one_side(
            &position, &lb_pair,
            &user_token_y, &reserve_y, &token_y_mint,
            &bin_array_lower, &bin_array_upper, &wallet,
            amount_y_raw,
            pool.active_id,
            Y_ONLY_SLIPPAGE, // matches the guard in liq_upper: liq_upper = active_id-1-S
                              // so all Y bins stay Y-side even with S-bin downward drift
            effective_liq_lower,
            effective_liq_upper,
        )
    } else {
        meteora_ix::ix_add_liquidity_by_strategy(
            &position, &lb_pair,
            &user_token_x, &user_token_y,
            &reserve_x, &reserve_y,
            &token_x_mint, &token_y_mint,
            &bin_array_lower, &bin_array_upper, &wallet,
            amount_x_raw, amount_y_raw,
            pool.active_id, args.bin_range,
            effective_liq_lower, effective_liq_upper,
        )
    };
    liq_ixs.push(liquidity_ix);

    let liq_b58 = meteora_ix::build_tx_b58(&liq_ixs, &wallet, blockhash)?;
    eprintln!("[liquidity] Submitting add_liquidity tx...");
    let liq_result = onchainos::contract_call_solana(&liq_b58, &meteora_ix::DLMM_PROGRAM.to_string())?;
    let liq_ok = liq_result["ok"].as_bool().unwrap_or(false)
        || liq_result["data"]["ok"].as_bool().unwrap_or(false);
    let liq_tx_hash = liq_result["data"]["txHash"]
        .as_str()
        .or_else(|| liq_result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string();

    let output = json!({
        "ok": liq_ok,
        "pool": args.pool,
        "wallet": wallet_str,
        "position": position.to_string(),
        "amount_x": args.amount_x,
        "amount_y": args.amount_y,
        "setup_tx_hash": if setup_tx_hash.is_empty() { serde_json::Value::Null } else { setup_tx_hash.clone().into() },
        "tx_hash": liq_tx_hash,
        "explorer_url": if !liq_tx_hash.is_empty() && liq_tx_hash != "pending" {
            format!("https://solscan.io/tx/{}", liq_tx_hash)
        } else {
            String::new()
        },
        "raw_result": liq_result,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
