use clap::Args;
use crate::config::{chain_config, token_symbol};
use crate::onchainos::resolve_wallet;
use crate::rpc::{format_amount, get_decimals, nft_balance_of, nft_token_of_owner_by_index, nfpm_positions};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (default: active onchainos wallet)
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn run(args: PositionsArgs, chain_id: u64) -> anyhow::Result<()> {
    let cfg = chain_config(chain_id)?;
    let rpc_owned = crate::config::rpc_url(chain_id)?;
    let rpc: &str = &rpc_owned;
    let nfpm = cfg.nfpm;

    let wallet = match args.wallet {
        Some(w) => w,
        None => resolve_wallet(chain_id)?,
    };

    let count = nft_balance_of(nfpm, &wallet, rpc).await?;
    if count == 0 {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "positions": [],
            "wallet":    wallet,
            "chain":     cfg.name,
            "message":   "No SushiSwap V3 LP positions found. Use `sushiswap-v3 mint-position` to open one."
        }))?);
        return Ok(());
    }

    let mut positions = Vec::new();
    for i in 0..count {
        let token_id = nft_token_of_owner_by_index(nfpm, &wallet, i, rpc).await?;
        let pos = match nfpm_positions(nfpm, token_id, rpc).await {
            Ok(p) => p,
            Err(_) => continue,
        };
        let sym0 = token_symbol(&pos.token0, chain_id);
        let sym1 = token_symbol(&pos.token1, chain_id);
        let dec0 = get_decimals(&pos.token0, rpc).await.unwrap_or(18);
        let dec1 = get_decimals(&pos.token1, rpc).await.unwrap_or(18);
        positions.push(serde_json::json!({
            "token_id":              token_id,
            "token0_symbol":         sym0,
            "token1_symbol":         sym1,
            "fee_bps":               pos.fee,
            "fee_pct":               format!("{:.4}%", pos.fee as f64 / 10_000.0),
            "tick_lower":            pos.tick_lower,
            "tick_upper":            pos.tick_upper,
            "liquidity":             pos.liquidity.to_string(),
            "in_range":              pos.liquidity > 0,
            "uncollected_fees_token0": format_amount(pos.tokens_owed0, dec0),
            "uncollected_fees_token1": format_amount(pos.tokens_owed1, dec1),
        }));
    }

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "chain":     cfg.name,
        "wallet":    wallet,
        "count":     positions.len(),
        "positions": positions,
    }))?);
    Ok(())
}
