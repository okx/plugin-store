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

    // ── 5. Compute position range (fixed width=70) and liquidity range ────────
    // DLMM positions always span MAX_BIN_PER_POSITION=70 bins.
    // The position is centered at the active bin (active_id - 35 to active_id + 34).
    // bin_range controls where liquidity is distributed within that window.
    const MAX_BIN_PER_POSITION: i32 = 70;
    let pos_lower = pool.active_id - MAX_BIN_PER_POSITION / 2; // active_id - 35
    let width = MAX_BIN_PER_POSITION;                           // 70
    let pos_upper = pos_lower + width - 1;                      // active_id + 34

    // Liquidity range: user-controlled via --bin-range (must fit inside position)
    anyhow::ensure!(
        args.bin_range <= 34,
        "--bin-range {} exceeds max 34 for a 70-bin position (active_id ± 34)",
        args.bin_range
    );
    let liq_lower = pool.active_id - args.bin_range;
    let liq_upper = pool.active_id + args.bin_range;

    // ── 6. Derive PDAs ───────────────────────────────────────────────────────
    let position = meteora_ix::position_pda(&lb_pair, &wallet, pos_lower, width);
    // Bin arrays cover the liquidity range.
    // DLMM requires bin_array_lower != bin_array_upper (program can't borrow same
    // account twice). If both bounds fall in the same bin array:
    //   1. Try extending liq_lower into the previous bin array (clamped to pos_lower).
    //   2. If pos_lower is in the same array, extend liq_upper into the next bin array
    //      (clamped to pos_upper) instead.
    let lower_idx_raw = meteora_ix::bin_array_index(liq_lower);
    let upper_idx_raw = meteora_ix::bin_array_index(liq_upper);
    let (lower_idx, upper_idx, effective_liq_lower, effective_liq_upper) =
        if lower_idx_raw == upper_idx_raw {
            // Attempt 1: extend liq_lower into previous bin array
            let prev_idx = upper_idx_raw - 1;
            let prev_last_bin = (prev_idx * 70 + 69) as i32;
            let adj_lower = prev_last_bin.max(pos_lower);
            let new_lower_idx = meteora_ix::bin_array_index(adj_lower);
            if new_lower_idx != upper_idx_raw {
                (new_lower_idx, upper_idx_raw, adj_lower, liq_upper)
            } else {
                // pos_lower is in the same bin array — extend liq_upper into next array
                let next_idx = upper_idx_raw + 1;
                let next_first_bin = (next_idx * 70) as i32;
                let adj_upper = next_first_bin.min(pos_upper);
                (lower_idx_raw, meteora_ix::bin_array_index(adj_upper), liq_lower, adj_upper)
            }
        } else {
            (lower_idx_raw, upper_idx_raw, liq_lower, liq_upper)
        };
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
    let ((token_x_acct, ata_x_exists), (token_y_acct, ata_y_exists), position_exists) =
        tokio::try_join!(
            solana_rpc::find_token_account(&client, &wallet_str, &mint_x_str2, &ata_x_str),
            solana_rpc::find_token_account(&client, &wallet_str, &mint_y_str2, &ata_y_str),
            solana_rpc::account_exists(&client, &pos_str),
        )?;
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

    // ATAs are created on-the-fly in the instruction list if missing.

    // ── 10-13. Build + submit with one automatic retry ───────────────────────
    // After closing a position, the Solana RPC may briefly return stale account
    // states (e.g. position still exists, or a bin array incorrectly missing).
    // If the first attempt fails with a simulation error, we wait 2 s, re-check
    // all mutable account states, rebuild the instruction list, and retry once.
    let bin_arr_lower_str = bin_array_lower.to_string();
    let bin_arr_upper_str = bin_array_upper.to_string();

    let mut last_result = serde_json::Value::Null;
    let mut last_ok = false;

    for attempt in 0u32..2 {
        if attempt > 0 {
            eprintln!("[retry] Simulation failed — waiting 2 s then re-checking account states...");
            tokio::time::sleep(std::time::Duration::from_millis(2_000)).await;
        }

        // Re-check mutable account states on every attempt so the instruction
        // list always reflects the current on-chain reality.
        let ba_lower_exists = solana_rpc::account_exists(&client, &bin_arr_lower_str).await?;
        let ba_upper_exists = solana_rpc::account_exists(&client, &bin_arr_upper_str).await?;
        let pos_exists_now = solana_rpc::account_exists(&client, &pos_str).await?;
        let blockhash = solana_rpc::get_latest_blockhash(&client).await?;

        eprintln!(
            "[attempt {}] bin_array_lower_exists={} bin_array_upper_exists={} position_exists={}",
            attempt + 1, ba_lower_exists, ba_upper_exists, pos_exists_now
        );

        let mut instructions = Vec::new();

        // Request extra compute budget — add_liquidity_by_strategy with position
        // init can exceed the default 200k CU limit.
        instructions.push(meteora_ix::ix_set_compute_unit_limit(600_000));

        // Create ATAs if missing (idempotent — safe to include even if they exist)
        if !ata_x_exists {
            instructions.push(meteora_ix::ix_create_ata_idempotent(
                &wallet, &user_token_x, &wallet, &token_x_mint,
            ));
        }
        if !ata_y_exists {
            instructions.push(meteora_ix::ix_create_ata_idempotent(
                &wallet, &user_token_y, &wallet, &token_y_mint,
            ));
        }

        // Wrap SOL → WSOL if token_x is the native SOL mint and amount_x > 0.
        // Transfers SOL to the WSOL ATA and syncs its token balance, ensuring
        // add_liquidity_by_strategy can debit the correct token amount.
        if token_x_mint == WSOL_MINT && amount_x_raw > 0 {
            instructions.push(meteora_ix::ix_sol_transfer(&wallet, &user_token_x, amount_x_raw));
            instructions.push(meteora_ix::ix_sync_native(&user_token_x));
        }
        if token_y_mint == WSOL_MINT && amount_y_raw > 0 {
            instructions.push(meteora_ix::ix_sol_transfer(&wallet, &user_token_y, amount_y_raw));
            instructions.push(meteora_ix::ix_sync_native(&user_token_y));
        }

        // Initialize bin arrays only if they genuinely don't exist.
        if !ba_lower_exists {
            instructions.push(meteora_ix::ix_initialize_bin_array(
                &lb_pair,
                &bin_array_lower,
                &wallet,
                lower_idx,
            ));
        }
        if lower_idx != upper_idx && !ba_upper_exists {
            instructions.push(meteora_ix::ix_initialize_bin_array(
                &lb_pair,
                &bin_array_upper,
                &wallet,
                upper_idx,
            ));
        }

        if !pos_exists_now {
            instructions.push(meteora_ix::ix_initialize_position_pda(
                &wallet,
                &lb_pair,
                &position,
                pos_lower,
                width,
            ));
        }

        instructions.push(meteora_ix::ix_add_liquidity_by_strategy(
            &position,
            &lb_pair,
            &user_token_x,
            &user_token_y,
            &reserve_x,
            &reserve_y,
            &token_x_mint,
            &token_y_mint,
            &bin_array_lower,
            &bin_array_upper,
            &wallet,
            amount_x_raw,
            amount_y_raw,
            pool.active_id,
            args.bin_range, // max_active_bin_slippage
            effective_liq_lower,
            effective_liq_upper,
        ));

        let tx_b58 = meteora_ix::build_tx_b58(&instructions, &wallet, blockhash)?;
        eprintln!("[debug] unsigned_tx_b58={}", &tx_b58[..32]);
        eprintln!("[debug] num_instructions={}", instructions.len());

        let result = onchainos::contract_call_solana(&tx_b58, &meteora_ix::DLMM_PROGRAM.to_string())?;
        let ok = result["ok"].as_bool().unwrap_or(false)
            || result["data"]["ok"].as_bool().unwrap_or(false);

        if ok {
            last_result = result;
            last_ok = true;
            break;
        }

        // Check if this is a simulation/transient error worth retrying.
        let err_str = result
            .get("error")
            .or_else(|| result["data"].get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let is_retryable = err_str.contains("simulation")
            || err_str.contains("ProgramAccountNotFound")
            || err_str.contains("BlockhashNotFound")
            || err_str.contains("stale");

        last_result = result;
        if !is_retryable || attempt >= 1 {
            break;
        }
        eprintln!("[retry] Retryable error detected: {err_str}");
    }

    let tx_hash = last_result["data"]["txHash"]
        .as_str()
        .or_else(|| last_result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string();

    let output = json!({
        "ok": last_ok,
        "pool": args.pool,
        "wallet": wallet_str,
        "position": position.to_string(),
        "amount_x": args.amount_x,
        "amount_y": args.amount_y,
        "tx_hash": tx_hash,
        "explorer_url": if !tx_hash.is_empty() && tx_hash != "pending" {
            format!("https://solscan.io/tx/{}", tx_hash)
        } else {
            String::new()
        },
        "raw_result": last_result,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
