use clap::Args;
use crate::config::{factory, resolve_token_validated, rpc_url, token_symbol};
use crate::rpc::{
    amm_get_pool, format_amount, get_decimals, get_total_supply,
    pool_get_reserves, pool_is_stable, pool_token0,
};

#[derive(Args)]
pub struct PoolsArgs {
    /// First token (symbol or address)
    #[arg(long)]
    pub token_a: String,
    /// Second token (symbol or address)
    #[arg(long)]
    pub token_b: String,
}

pub async fn run(args: PoolsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let fac = factory();
    let token_a = resolve_token_validated(&args.token_a)?;
    let token_b = resolve_token_validated(&args.token_b)?;

    let sym_a = resolve_symbol(&token_a, &args.token_a);
    let sym_b = resolve_symbol(&token_b, &args.token_b);

    eprintln!("Searching Aerodrome AMM pools for {}/{} on Base...", sym_a, sym_b);

    let zero = "0x0000000000000000000000000000000000000000";
    let mut results = vec![];

    for is_stable in [false, true] {
        let pool = amm_get_pool(fac, &token_a, &token_b, is_stable, rpc).await?;
        if pool == zero { continue; }

        // Verify pool and get token ordering
        let t0 = pool_token0(&pool, rpc).await.unwrap_or_default();
        let confirmed_stable = pool_is_stable(&pool, rpc).await.unwrap_or(is_stable);
        let (r0, r1) = pool_get_reserves(&pool, rpc).await.unwrap_or((0, 0));
        let total_supply = get_total_supply(&pool, rpc).await.unwrap_or(0);

        // Determine which reserve corresponds to which token
        let (sym_0, sym_1, res_0, res_1, dec_0, dec_1) = if t0.to_lowercase() == token_a.to_lowercase() {
            let d0 = get_decimals(&token_a, rpc).await.unwrap_or(18);
            let d1 = get_decimals(&token_b, rpc).await.unwrap_or(18);
            (sym_a.clone(), sym_b.clone(), r0, r1, d0, d1)
        } else {
            let d0 = get_decimals(&token_b, rpc).await.unwrap_or(18);
            let d1 = get_decimals(&token_a, rpc).await.unwrap_or(18);
            (sym_b.clone(), sym_a.clone(), r0, r1, d0, d1)
        };

        // Compute price: how much token_1 per token_0
        let price = if res_0 > 0 {
            let r0f = res_0 as f64 / 10f64.powi(dec_0 as i32);
            let r1f = res_1 as f64 / 10f64.powi(dec_1 as i32);
            format!("{:.6}", r1f / r0f)
        } else {
            "0".to_string()
        };

        results.push(serde_json::json!({
            "pool": pool,
            "pool_type": if confirmed_stable { "stable" } else { "volatile" },
            "token0": sym_0,
            "token1": sym_1,
            "reserve0": format_amount(res_0, dec_0),
            "reserve1": format_amount(res_1, dec_1),
            "price_token1_per_token0": price,
            "total_lp_supply": format_amount(total_supply, 18),
            "chain": "Base (8453)"
        }));
    }

    if results.is_empty() {
        println!(
            "No Aerodrome AMM pools found for {}/{}.\n\
             Tip: For concentrated liquidity, try `aerodrome-slipstream pools`.",
            sym_a, sym_b
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&results)?);
    }
    Ok(())
}

fn resolve_symbol(addr: &str, fallback: &str) -> String {
    let s = token_symbol(addr);
    if s != "UNKNOWN" { s.to_string() } else { fallback.to_string() }
}
