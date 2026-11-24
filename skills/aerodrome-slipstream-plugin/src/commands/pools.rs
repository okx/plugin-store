use clap::Args;
use crate::config::{cl_factory, common_tick_spacings, resolve_token, rpc_url, token_symbol};
use crate::rpc::{cl_get_pool, get_decimals, pool_fee, pool_liquidity, pool_tick_spacing, sqrt_price_to_human, pool_slot0};

#[derive(Args)]
pub struct PoolsArgs {
    /// First token (symbol or address, e.g. WETH, USDC)
    #[arg(long)]
    pub token_a: String,
    /// Second token (symbol or address)
    #[arg(long)]
    pub token_b: String,
}

pub async fn run(args: PoolsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let factory = cl_factory();
    let token_a = resolve_token(&args.token_a);
    let token_b = resolve_token(&args.token_b);

    let sym_a = if token_symbol(&token_a) != "UNKNOWN" { token_symbol(&token_a).to_string() } else { token_a[..6].to_string() + "..." };
    let sym_b = if token_symbol(&token_b) != "UNKNOWN" { token_symbol(&token_b).to_string() } else { token_b[..6].to_string() + "..." };

    println!("Searching Aerodrome Slipstream CL pools for {}/{} on Base...", sym_a, sym_b);

    // Determine token0/token1 order by address comparison (Uniswap V3 / Aerodrome CL invariant).
    // The factory always stores the pool with token0 < token1 lexicographically.
    let (token0, token1, dec0, dec1) = if token_a.to_lowercase() < token_b.to_lowercase() {
        let d0 = get_decimals(&token_a, rpc).await.unwrap_or(18);
        let d1 = get_decimals(&token_b, rpc).await.unwrap_or(18);
        (token_a.clone(), token_b.clone(), d0, d1)
    } else {
        let d0 = get_decimals(&token_b, rpc).await.unwrap_or(18);
        let d1 = get_decimals(&token_a, rpc).await.unwrap_or(18);
        (token_b.clone(), token_a.clone(), d0, d1)
    };

    let mut found = 0;
    let mut results = vec![];

    for &ts in common_tick_spacings() {
        let pool = cl_get_pool(factory, &token_a, &token_b, ts, rpc).await?;
        let zero = "0x0000000000000000000000000000000000000000";
        if pool == zero || pool.to_lowercase() == zero {
            continue;
        }

        let liq = pool_liquidity(&pool, rpc).await.unwrap_or(0);
        let fee = pool_fee(&pool, rpc).await.unwrap_or(0);
        let actual_ts = pool_tick_spacing(&pool, rpc).await.unwrap_or(ts);

        let price = if let Ok((sp, _)) = pool_slot0(&pool, rpc).await {
            sqrt_price_to_human(sp, dec0, dec1)
        } else { 0.0 };

        let fee_pct = fee as f64 / 10000.0;
        results.push(serde_json::json!({
            "pool": pool,
            "tick_spacing": actual_ts,
            "fee_bps": fee,
            "fee_pct": format!("{:.4}%", fee_pct),
            "liquidity": liq.to_string(),
            "price_token1_per_token0": format!("{:.6}", price),
            "token0": token0,
            "token1": token1,
        }));
        found += 1;
    }

    if found == 0 {
        println!("No Slipstream CL pools found for {}/{}.", sym_a, sym_b);
        println!("Tip: check if these tokens have liquidity in Aerodrome AMM (classic pools) instead.");
    } else {
        println!("{}", serde_json::to_string_pretty(&results)?);
    }
    Ok(())
}
