use clap::Args;
use reqwest::Client;
use serde_json::json;

use crate::api::MeteoraClient;
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
    // Resolve wallet address
    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!("Cannot resolve wallet address. Pass --wallet <address> or log in via onchainos.\nError: {e}")
        })?
    };

    if wallet.is_empty() {
        anyhow::bail!("Wallet address is empty. Pass --wallet <address> or log in via onchainos.");
    }

    // ── 1. Try Meteora REST API ──────────────────────────────────────────────
    let api_client = MeteoraClient::new();
    let mut api_positions = api_client.get_positions(&wallet).await?;

    // Filter by pool if specified
    if let Some(pool_addr) = &args.pool {
        api_positions.retain(|p| p.pair_address == *pool_addr);
    }

    if !api_positions.is_empty() {
        // API returned data — use it (has USD values, fees, bin amounts)
        let total_value_usd: f64 = api_positions.iter().map(|p| p.total_value_usd).sum();
        let total_fee_usd: f64 = api_positions.iter().map(|p| p.total_fee_usd).sum();

        let positions_out: Vec<serde_json::Value> = api_positions
            .iter()
            .map(|p| {
                json!({
                    "position_address": p.address,
                    "pool_address": p.pair_address,
                    "owner": p.owner,
                    "token_x_amount": p.total_x_amount,
                    "token_y_amount": p.total_y_amount,
                    "fee_x_unclaimed": p.fee_x,
                    "fee_y_unclaimed": p.fee_y,
                    "total_fee_usd": p.total_fee_usd,
                    "total_value_usd": p.total_value_usd,
                    "bin_range": {
                        "lower_bin_id": p.lower_bin_id,
                        "upper_bin_id": p.upper_bin_id,
                    },
                    "bin_data_count": p.data.len(),
                    "source": "api",
                })
            })
            .collect();

        let output = json!({
            "ok": true,
            "wallet": wallet,
            "positions_count": positions_out.len(),
            "summary": {
                "total_value_usd": total_value_usd,
                "total_unclaimed_fees_usd": total_fee_usd,
            },
            "positions": positions_out,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // ── 2. API returned empty — fall back to on-chain RPC ───────────────────
    eprintln!("[info] Meteora API returned no positions; querying on-chain via getProgramAccounts...");

    let rpc_client = Client::new();
    let chain_positions = solana_rpc::get_dlmm_positions_by_owner(
        &rpc_client,
        &DLMM_PROGRAM.to_string(),
        &wallet,
        args.pool.as_deref(),
    )
    .await?;

    if chain_positions.is_empty() {
        let output = json!({
            "ok": true,
            "wallet": wallet,
            "positions": [],
            "message": "No positions found for this wallet (checked API and on-chain)",
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // On-chain data has no USD values — return structural info only
    let positions_out: Vec<serde_json::Value> = chain_positions
        .iter()
        .map(|p| {
            json!({
                "position_address": p.address,
                "pool_address": p.lb_pair,
                "owner": p.owner,
                "bin_range": {
                    "lower_bin_id": p.lower_bin_id,
                    "upper_bin_id": p.upper_bin_id,
                },
                "source": "on-chain",
                "note": "USD values unavailable (API indexing delay); use position_address with remove-liquidity --position",
            })
        })
        .collect();

    let output = json!({
        "ok": true,
        "wallet": wallet,
        "positions_count": positions_out.len(),
        "source": "on-chain",
        "note": "Meteora API has not yet indexed these positions. Bin range and addresses are accurate.",
        "positions": positions_out,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
