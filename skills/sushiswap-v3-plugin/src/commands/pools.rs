use clap::Args;
use crate::config::{chain_config, common_fee_tiers, fee_to_tick_spacing, resolve_token, token_symbol};
use crate::rpc::{get_decimals, pool_fee, pool_liquidity, pool_slot0, pool_token0, sqrt_price_to_human, v3_get_pool};

#[derive(Args)]
pub struct PoolsArgs {
    /// First token (symbol or address, e.g. WETH, USDC)
    #[arg(long)]
    pub token_a: String,
    /// Second token (symbol or address)
    #[arg(long)]
    pub token_b: String,
}

pub async fn run(args: PoolsArgs, chain_id: u64) -> anyhow::Result<()> {
    let cfg = chain_config(chain_id)?;
    let rpc_owned = crate::config::rpc_url(chain_id)?;
    let rpc: &str = &rpc_owned;
    let token_a = resolve_token(&args.token_a, chain_id);
    let token_b = resolve_token(&args.token_b, chain_id);
    let zero = "0x0000000000000000000000000000000000000000";

    eprintln!("[sushiswap-v3] Searching SushiSwap V3 pools for {}/{} on {}...",
        args.token_a, args.token_b, cfg.name);

    let mut pools = Vec::new();
    for &fee in common_fee_tiers() {
        let pool = v3_get_pool(cfg.factory, &token_a, &token_b, fee, rpc).await?;
        if pool == zero { continue; }

        let (sqrt_price, _tick) = pool_slot0(&pool, rpc).await.unwrap_or((0, 0));
        let liquidity = pool_liquidity(&pool, rpc).await.unwrap_or(0);
        let actual_fee = pool_fee(&pool, rpc).await.unwrap_or(fee);
        let t0 = pool_token0(&pool, rpc).await.unwrap_or_default();
        let sym0 = token_symbol(&t0, chain_id);
        let t1_addr = if t0.to_lowercase() == token_a.to_lowercase() { &token_b } else { &token_a };
        let sym1 = token_symbol(t1_addr, chain_id);
        let dec0 = get_decimals(&t0, rpc).await.unwrap_or(18);
        let dec1 = get_decimals(t1_addr, rpc).await.unwrap_or(18);
        let price = sqrt_price_to_human(sqrt_price, dec0, dec1);

        pools.push(serde_json::json!({
            "pool":          pool,
            "chain":         cfg.name,
            "token0":        sym0,
            "token1":        sym1,
            "fee_bps":       actual_fee,
            "fee_pct":       format!("{:.4}%", actual_fee as f64 / 10_000.0),
            "tick_spacing":  fee_to_tick_spacing(actual_fee),
            "liquidity":     liquidity.to_string(),
            "price_token1_per_token0": format!("{:.6}", price),
        }));
    }

    if pools.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "pools": [],
            "message": format!("No SushiSwap V3 pools found for {}/{} on {}.",
                args.token_a, args.token_b, cfg.name)
        }))?);
    } else {
        println!("{}", serde_json::to_string_pretty(&pools)?);
    }
    Ok(())
}
