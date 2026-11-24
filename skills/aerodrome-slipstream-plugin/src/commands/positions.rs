use clap::Args;
use crate::config::{cl_factory, nfpm, rpc_url, token_symbol, CHAIN_ID};
use crate::onchainos::resolve_wallet;
use crate::rpc::{cl_get_pool, get_decimals, format_amount, nft_balance_of, nft_token_of_owner_by_index, nfpm_positions, pool_slot0};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address (default: active onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let nfpm_addr = nfpm();

    let owner = match args.wallet {
        Some(w) => w,
        None => resolve_wallet(CHAIN_ID)?,
    };

    if !owner.starts_with("0x") || owner.len() != 42 {
        anyhow::bail!("Invalid wallet address '{}'. Expected a 0x-prefixed 20-byte hex address (42 chars).", owner);
    }

    println!("Fetching Aerodrome Slipstream positions for {}...", &owner[..10]);

    let count = nft_balance_of(nfpm_addr, &owner, rpc).await?;
    if count == 0 {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "wallet": owner,
            "positions": [],
            "message": "No Slipstream LP positions found."
        }))?);
        return Ok(());
    }

    let mut positions = vec![];
    for i in 0..count {
        let token_id = nft_token_of_owner_by_index(nfpm_addr, &owner, i, rpc).await?;
        match nfpm_positions(nfpm_addr, token_id, rpc).await {
            Ok(pos) => {
                let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
                let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);
                let sym0 = token_symbol(&pos.token0);
                let sym1 = token_symbol(&pos.token1);

                // Look up current tick for in_range check
                let in_range = if let Ok(pool) = cl_get_pool(cl_factory(), &pos.token0, &pos.token1, pos.tick_spacing, rpc).await {
                    if let Ok((_, current_tick)) = pool_slot0(&pool, rpc).await {
                        current_tick >= pos.tick_lower && current_tick < pos.tick_upper
                    } else { false }
                } else { false };

                positions.push(serde_json::json!({
                    "token_id": token_id,
                    "token0": pos.token0,
                    "token0_symbol": sym0,
                    "token1": pos.token1,
                    "token1_symbol": sym1,
                    "tick_spacing": pos.tick_spacing,
                    "tick_lower": pos.tick_lower,
                    "tick_upper": pos.tick_upper,
                    "liquidity": pos.liquidity.to_string(),
                    "in_range": in_range,
                    "uncollected_fees_token0": format_amount(pos.tokens_owed0, dec0),
                    "uncollected_fees_token1": format_amount(pos.tokens_owed1, dec1),
                }));
            }
            Err(e) => {
                eprintln!("Warning: could not fetch position {}: {}", token_id, e);
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "wallet": owner,
        "count": count,
        "positions": positions,
        "chain": "Base (8453)"
    }))?);
    Ok(())
}
