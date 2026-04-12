use clap::Args;
use reqwest::Client;
use serde_json::json;
use solana_pubkey::Pubkey;
use std::str::FromStr;

use crate::meteora_ix;
use crate::onchainos;
use crate::solana_rpc;

#[derive(Args, Debug)]
pub struct RemoveLiquidityArgs {
    /// Meteora DLMM pool (LbPair) address
    #[arg(long)]
    pub pool: String,

    /// Position PDA address (from add-liquidity output or get-user-positions)
    #[arg(long)]
    pub position: String,

    /// Percentage of liquidity to remove, 1–100 (default: 100)
    #[arg(long, default_value = "100")]
    pub pct: u8,

    /// Also close the position account after removing liquidity (reclaims ~0.057 SOL rent)
    #[arg(long)]
    pub close: bool,

    /// Wallet address. If omitted, uses the currently logged-in onchainos wallet.
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(args: &RemoveLiquidityArgs, dry_run: bool) -> anyhow::Result<()> {
    let client = Client::new();

    // ── 1. Resolve wallet ────────────────────────────────────────────────────
    let wallet_str = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!("Cannot resolve wallet. Pass --wallet or log in via onchainos.\nError: {e}")
        })?
    };
    let wallet = Pubkey::from_str(&wallet_str)?;
    let lb_pair = Pubkey::from_str(&args.pool)?;
    let position = Pubkey::from_str(&args.position)?;

    anyhow::ensure!(args.pct >= 1 && args.pct <= 100, "--pct must be 1–100");
    let bps_to_remove: u16 = (args.pct as u16) * 100; // 100% → 10000 bps

    // ── 2. Fetch & parse LbPair ─────────────────────────────────────────────
    let pool_data = solana_rpc::get_account_data(&client, &args.pool).await?;
    let pool = solana_rpc::parse_lb_pair(&pool_data)?;

    let token_x_mint = Pubkey::from(pool.token_x_mint);
    let token_y_mint = Pubkey::from(pool.token_y_mint);
    let reserve_x = Pubkey::from(pool.reserve_x);
    let reserve_y = Pubkey::from(pool.reserve_y);

    // ── 3. Parse position account to get bin range ──────────────────────────
    let pos_data = solana_rpc::get_account_data(&client, &args.position).await?;
    let (lower_bin_id, upper_bin_id) = solana_rpc::parse_position_bins(&pos_data)?;

    // Guard: reject empty positions early (all 70 liquidity shares == 0).
    // DLMM returns Custom(6002) InvalidInput when remove is attempted on an empty position.
    // Exception: if --close is set, we can still close the empty position to reclaim rent.
    let has_liquidity = solana_rpc::position_has_liquidity(&pos_data);
    if !has_liquidity {
        if !args.close {
            let output = serde_json::json!({
                "ok": false,
                "error": "Position has no liquidity to remove.",
                "position": args.position,
                "lower_bin_id": lower_bin_id,
                "upper_bin_id": upper_bin_id,
                "tip": "Run with --close to close the empty position and reclaim ~0.057 SOL rent."
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }
        // Empty position + --close: claim any pending fees, then close.
        // close_position requires fee_pending fields to be zero first.
        let lower_idx = meteora_ix::bin_array_index(lower_bin_id);
        let upper_idx = meteora_ix::bin_array_index(upper_bin_id);
        let bin_array_lower = meteora_ix::bin_array_pda(&lb_pair, lower_idx);
        let bin_array_upper = meteora_ix::bin_array_pda(&lb_pair, upper_idx);

        // Resolve token accounts (needed for claim_fee)
        let ata_x = meteora_ix::get_ata(&wallet, &token_x_mint);
        let ata_y = meteora_ix::get_ata(&wallet, &token_y_mint);
        let ata_x_str = ata_x.to_string();
        let ata_y_str = ata_y.to_string();
        let mint_x_str = token_x_mint.to_string();
        let mint_y_str = token_y_mint.to_string();
        let (user_token_x, ata_x_exists) =
            solana_rpc::find_token_account(&client, &wallet_str, &mint_x_str, &ata_x_str).await?;
        let (user_token_y, ata_y_exists) =
            solana_rpc::find_token_account(&client, &wallet_str, &mint_y_str, &ata_y_str).await?;
        let user_token_x_pk: Pubkey = if ata_x_exists { user_token_x.parse()? } else { ata_x };
        let user_token_y_pk: Pubkey = if ata_y_exists { user_token_y.parse()? } else { ata_y };

        if dry_run {
            let output = serde_json::json!({
                "ok": true,
                "dry_run": true,
                "message": "Dry run: position is empty, will claim pending fees then close account.",
                "position": args.position,
                "lower_bin_id": lower_bin_id,
                "upper_bin_id": upper_bin_id,
                "will_close_position": true,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }
        let blockhash = solana_rpc::get_latest_blockhash(&client).await?;
        let mut instructions = Vec::new();
        if !ata_x_exists {
            instructions.push(meteora_ix::ix_create_ata_idempotent(&wallet, &ata_x, &wallet, &token_x_mint));
        }
        if !ata_y_exists {
            instructions.push(meteora_ix::ix_create_ata_idempotent(&wallet, &ata_y, &wallet, &token_y_mint));
        }
        instructions.push(meteora_ix::ix_claim_fee(
            &lb_pair, &position, &bin_array_lower, &bin_array_upper,
            &wallet, &reserve_x, &reserve_y,
            &user_token_x_pk, &user_token_y_pk,
            &token_x_mint, &token_y_mint,
        ));
        instructions.push(meteora_ix::ix_close_position_if_empty(&wallet, &position));
        instructions.push(meteora_ix::ix_set_compute_unit_limit(400_000));
        let instructions = instructions;
        let tx_b58 = meteora_ix::build_tx_b58(&instructions, &wallet, blockhash)?;
        eprintln!("[debug] close-only unsigned_tx_b58={}...", &tx_b58[..32]);
        let result = onchainos::contract_call_solana(&tx_b58, &meteora_ix::DLMM_PROGRAM.to_string())?;
        let tx_hash = onchainos::extract_tx_hash(&result);
        let ok = result["ok"].as_bool().unwrap_or(false)
            || result["data"]["ok"].as_bool().unwrap_or(false)
            || !tx_hash.is_empty() && tx_hash != "pending";
        let output = serde_json::json!({
            "ok": ok,
            "position": args.position,
            "wallet": wallet_str,
            "position_closed": ok,
            "tx_hash": tx_hash,
            "explorer_url": if !tx_hash.is_empty() && tx_hash != "pending" {
                format!("https://solscan.io/tx/{}", tx_hash)
            } else { String::new() },
            "raw_result": result,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // ── 4. Resolve token accounts ────────────────────────────────────────────
    let ata_x = meteora_ix::get_ata(&wallet, &token_x_mint);
    let ata_y = meteora_ix::get_ata(&wallet, &token_y_mint);
    let ata_x_str = ata_x.to_string();
    let ata_y_str = ata_y.to_string();
    let mint_x_str = token_x_mint.to_string();
    let mint_y_str = token_y_mint.to_string();
    let (user_token_x, ata_x_exists) =
        solana_rpc::find_token_account(&client, &wallet_str, &mint_x_str, &ata_x_str).await?;
    let (user_token_y, ata_y_exists) =
        solana_rpc::find_token_account(&client, &wallet_str, &mint_y_str, &ata_y_str).await?;

    // ── 5. Derive bin array PDAs ─────────────────────────────────────────────
    let lower_idx = meteora_ix::bin_array_index(lower_bin_id);
    let upper_idx = meteora_ix::bin_array_index(upper_bin_id);
    let bin_array_lower = meteora_ix::bin_array_pda(&lb_pair, lower_idx);
    let bin_array_upper = meteora_ix::bin_array_pda(&lb_pair, upper_idx);

    // ── 6. Dry-run output ────────────────────────────────────────────────────
    if dry_run {
        let output = json!({
            "ok": true,
            "dry_run": true,
            "message": "Dry run: preview only, no transaction submitted.",
            "pool": args.pool,
            "position": args.position,
            "wallet": wallet_str,
            "lower_bin_id": lower_bin_id,
            "upper_bin_id": upper_bin_id,
            "pct": args.pct,
            "bps_to_remove": bps_to_remove,
            "will_claim_fees_then_close": args.close && bps_to_remove == 10000,
            "token_x_mint": token_x_mint.to_string(),
            "token_y_mint": token_y_mint.to_string(),
            "user_token_x": user_token_x,
            "user_token_y": user_token_y,
            "bin_array_lower_pda": bin_array_lower.to_string(),
            "bin_array_upper_pda": bin_array_upper.to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // ── 7. Get blockhash ─────────────────────────────────────────────────────
    let blockhash = solana_rpc::get_latest_blockhash(&client).await?;

    // ── 8. Build instructions ────────────────────────────────────────────────
    // If the ATA didn't exist, we're creating it — use the ATA address as the destination
    let user_token_x_pk: Pubkey = if ata_x_exists { user_token_x.parse()? } else { ata_x };
    let user_token_y_pk: Pubkey = if ata_y_exists { user_token_y.parse()? } else { ata_y };

    let mut instructions = Vec::new();

    // Create ATAs if they don't exist (tokens will be received here)
    if !ata_x_exists {
        instructions.push(meteora_ix::ix_create_ata_idempotent(
            &wallet, &ata_x, &wallet, &token_x_mint,
        ));
    }
    if !ata_y_exists {
        instructions.push(meteora_ix::ix_create_ata_idempotent(
            &wallet, &ata_y, &wallet, &token_y_mint,
        ));
    }

    instructions.push(meteora_ix::ix_remove_liquidity_by_range(
        &position,
        &lb_pair,
        &user_token_x_pk,
        &user_token_y_pk,
        &reserve_x,
        &reserve_y,
        &token_x_mint,
        &token_y_mint,
        &bin_array_lower,
        &bin_array_upper,
        &wallet,
        lower_bin_id,
        upper_bin_id,
        bps_to_remove,
    ));

    if args.close && bps_to_remove == 10000 {
        // Claim pending fees first — close_position_if_empty requires fee_infos == 0,
        // which removeLiquidityByRange does NOT clear automatically.
        instructions.push(meteora_ix::ix_claim_fee(
            &lb_pair, &position, &bin_array_lower, &bin_array_upper,
            &wallet, &reserve_x, &reserve_y,
            &user_token_x_pk, &user_token_y_pk,
            &token_x_mint, &token_y_mint,
        ));
        instructions.push(meteora_ix::ix_close_position_if_empty(&wallet, &position));
    }

    // Request 600k CUs — placed last so onchainos sees the DLMM instruction first.
    // Solana runtime processes budget instructions regardless of position.
    instructions.push(meteora_ix::ix_set_compute_unit_limit(600_000));

    // ── 9. Build & submit tx ─────────────────────────────────────────────────
    let tx_b58 = meteora_ix::build_tx_b58(&instructions, &wallet, blockhash)?;
    eprintln!("[debug] unsigned_tx_b58={}...", &tx_b58[..32]);
    eprintln!("[debug] num_instructions={}", instructions.len());

    let result = onchainos::contract_call_solana(&tx_b58, &meteora_ix::DLMM_PROGRAM.to_string())?;
    let tx_hash = onchainos::extract_tx_hash(&result);
    let ok = result["ok"].as_bool().unwrap_or(false)
        || result["data"]["ok"].as_bool().unwrap_or(false)
        || !tx_hash.is_empty() && tx_hash != "pending";

    let output = json!({
        "ok": ok,
        "pool": args.pool,
        "position": args.position,
        "wallet": wallet_str,
        "pct_removed": args.pct,
        "position_closed": args.close && ok,
        "tx_hash": tx_hash,
        "explorer_url": if !tx_hash.is_empty() && tx_hash != "pending" {
            format!("https://solscan.io/tx/{}", tx_hash)
        } else { String::new() },
        "raw_result": result,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
