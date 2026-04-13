use clap::Args;
use reqwest::Client;
use serde_json::json;
use solana_pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

use crate::meteora_ix;
use crate::meteora_ix::DLMM_PROGRAM;
use crate::onchainos;
use crate::solana_rpc;

#[derive(Args, Debug)]
pub struct GetUserPositionsArgs {
    /// Wallet address (Solana pubkey). If omitted, uses the currently logged-in wallet.
    #[arg(long)]
    pub wallet: Option<String>,

    /// Filter by pool address (optional)
    #[arg(long)]
    pub pool: Option<String>,
}

pub async fn execute(args: &GetUserPositionsArgs) -> anyhow::Result<()> {
    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!(
                "Cannot resolve wallet address. Pass --wallet <address> or log in via onchainos.\nError: {e}"
            )
        })?
    };

    if wallet.is_empty() {
        anyhow::bail!("Wallet address is empty. Pass --wallet <address> or log in via onchainos.");
    }

    let client = Client::new();

    eprintln!("[info] Querying on-chain positions via getProgramAccounts...");
    let chain_positions = solana_rpc::get_dlmm_positions_by_owner(
        &client,
        &DLMM_PROGRAM.to_string(),
        &wallet,
        args.pool.as_deref(),
    )
    .await?;

    if chain_positions.is_empty() {
        let output = json!({
            "ok": true,
            "wallet": wallet,
            "positions_count": 0,
            "positions": [],
            "message": "No DLMM positions found for this wallet.",
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let mut positions_out: Vec<serde_json::Value> = Vec::new();
    for pos in &chain_positions {
        match enrich_position(&client, pos).await {
            Ok(v) => positions_out.push(v),
            Err(e) => {
                eprintln!("[warn] Failed to enrich position {}: {e}", pos.address);
                positions_out.push(json!({
                    "position_address": pos.address,
                    "pool_address": pos.lb_pair,
                    "owner": pos.owner,
                    "bin_range": {
                        "lower_bin_id": pos.lower_bin_id,
                        "upper_bin_id": pos.upper_bin_id,
                    },
                    "error": e.to_string(),
                    "source": "on-chain",
                }));
            }
        }
    }

    let output = json!({
        "ok": true,
        "wallet": wallet,
        "positions_count": positions_out.len(),
        "note": "Token amounts are estimated from on-chain BinArray state and may differ slightly from exact withdrawable amounts due to rounding and in-flight trades.",
        "positions": positions_out,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn enrich_position(
    client: &Client,
    pos: &solana_rpc::OnChainPosition,
) -> anyhow::Result<serde_json::Value> {
    // Fetch LbPair and position account data concurrently
    let (pool_data, pos_data) = tokio::try_join!(
        solana_rpc::get_account_data(client, &pos.lb_pair),
        solana_rpc::get_account_data(client, &pos.address),
    )?;

    let pool = solana_rpc::parse_lb_pair(&pool_data)?;
    let shares = solana_rpc::parse_position_shares(&pos_data);

    let mint_x = bs58::encode(&pool.token_x_mint).into_string();
    let mint_y = bs58::encode(&pool.token_y_mint).into_string();

    // Fetch token decimals concurrently
    let (mint_x_data, mint_y_data) = tokio::try_join!(
        solana_rpc::get_account_data(client, &mint_x),
        solana_rpc::get_account_data(client, &mint_y),
    )?;
    let decimals_x = solana_rpc::parse_mint_decimals(&mint_x_data);
    let decimals_y = solana_rpc::parse_mint_decimals(&mint_y_data);

    let lower_bin_id = pos.lower_bin_id;

    // Identify which BinArray accounts are needed
    let mut needed_arrays: Vec<i64> = Vec::new();
    for i in 0..70usize {
        if shares[i] == 0 {
            continue;
        }
        let bin_id = lower_bin_id + i as i32;
        let arr_idx = meteora_ix::bin_array_index(bin_id);
        if !needed_arrays.contains(&arr_idx) {
            needed_arrays.push(arr_idx);
        }
    }

    // Fetch each BinArray (typically 1-2 per position)
    let lb_pair_key = Pubkey::from_str(&pos.lb_pair)?;
    let mut bin_arrays: HashMap<i64, Vec<u8>> = HashMap::new();
    for arr_idx in &needed_arrays {
        let ba_addr = meteora_ix::bin_array_pda(&lb_pair_key, *arr_idx).to_string();
        match solana_rpc::get_account_data(client, &ba_addr).await {
            Ok(data) => {
                bin_arrays.insert(*arr_idx, data);
            }
            Err(e) => {
                eprintln!("[warn] BinArray index {arr_idx} fetch failed: {e}");
            }
        }
    }

    // Compute user token amounts via proportional share formula:
    //   user_amount = bin_amount × user_shares / bin_liquidity_supply
    let mut total_x: f64 = 0.0;
    let mut total_y: f64 = 0.0;
    let mut active_bins: u32 = 0;

    for i in 0..70usize {
        if shares[i] == 0 {
            continue;
        }
        let bin_id = lower_bin_id + i as i32;
        let arr_idx = meteora_ix::bin_array_index(bin_id);
        let pos_in_array = (bin_id as i64 - arr_idx * 70) as usize;

        if let Some(ba_data) = bin_arrays.get(&arr_idx) {
            let (amount_x, amount_y, supply) = solana_rpc::parse_bin_at(ba_data, pos_in_array);
            if supply > 0 {
                let fraction = shares[i] as f64 / supply as f64;
                total_x += amount_x as f64 * fraction;
                total_y += amount_y as f64 * fraction;
                active_bins += 1;
            }
        }
    }

    let token_x_amount = total_x / 10f64.powi(decimals_x as i32);
    let token_y_amount = total_y / 10f64.powi(decimals_y as i32);

    Ok(json!({
        "position_address": pos.address,
        "pool_address": pos.lb_pair,
        "owner": pos.owner,
        "token_x_mint": mint_x,
        "token_y_mint": mint_y,
        "token_x_amount": token_x_amount,
        "token_y_amount": token_y_amount,
        "token_x_decimals": decimals_x,
        "token_y_decimals": decimals_y,
        "bin_range": {
            "lower_bin_id": lower_bin_id,
            "upper_bin_id": pos.upper_bin_id,
        },
        "active_bins": active_bins,
        "source": "on-chain",
    }))
}
