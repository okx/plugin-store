use clap::Args;
use crate::config::{cl_factory, common_tick_spacings, resolve_token, rpc_url, token_symbol};
use crate::rpc::{cl_get_pool, get_decimals, pool_slot0, sqrt_price_to_human, pool_liquidity};

#[derive(Args)]
pub struct PricesArgs {
    /// Base token (e.g. WETH)
    #[arg(long)]
    pub token_in: String,
    /// Quote token (e.g. USDC)
    #[arg(long)]
    pub token_out: String,
    /// Tick spacing to use (default: auto-select most liquid)
    #[arg(long)]
    pub tick_spacing: Option<i32>,
}

pub async fn run(args: PricesArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let factory = cl_factory();
    let token_in = resolve_token(&args.token_in);
    let token_out = resolve_token(&args.token_out);

    let sym_in  = if token_symbol(&token_in) != "UNKNOWN"  { token_symbol(&token_in).to_string()  } else { args.token_in.clone() };
    let sym_out = if token_symbol(&token_out) != "UNKNOWN" { token_symbol(&token_out).to_string() } else { args.token_out.clone() };

    let dec_in  = get_decimals(&token_in, rpc).await.unwrap_or(18);
    let dec_out = get_decimals(&token_out, rpc).await.unwrap_or(6);

    let tick_spacings: Vec<i32> = match args.tick_spacing {
        Some(ts) => vec![ts],
        None => common_tick_spacings().to_vec(),
    };

    let zero = "0x0000000000000000000000000000000000000000";
    let mut best_pool = String::new();
    let mut best_liq: u128 = 0;
    let mut best_ts = 0i32;

    for ts in &tick_spacings {
        let pool = cl_get_pool(factory, &token_in, &token_out, *ts, rpc).await?;
        if pool == zero { continue; }
        let liq = pool_liquidity(&pool, rpc).await.unwrap_or(0);
        if liq > best_liq {
            best_liq = liq;
            best_pool = pool.clone();
            best_ts = *ts;
        }
    }

    if best_pool.is_empty() {
        anyhow::bail!("No Slipstream CL pool found for {}/{}", sym_in, sym_out);
    }

    let (sqrt_price, tick) = pool_slot0(&best_pool, rpc).await?;

    // Determine if token_in is token0 or token1 of the pool
    // The price from slot0 is always token1/token0. If token_in is token1, invert.
    use crate::rpc::pool_token0;
    let pool_t0 = pool_token0(&best_pool, rpc).await?;
    let (d0, d1, invert) = if pool_t0.to_lowercase() == token_in.to_lowercase() {
        (dec_in, dec_out, false)
    } else {
        (dec_out, dec_in, true)
    };

    let price_raw = sqrt_price_to_human(sqrt_price, d0, d1);
    let price = if invert && price_raw > 0.0 { 1.0 / price_raw } else { price_raw };

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "pair": format!("{}/{}", sym_in, sym_out),
        "price": format!("{:.6} {} per {}", price, sym_out, sym_in),
        "pool": best_pool,
        "tick_spacing": best_ts,
        "current_tick": tick,
        "liquidity": best_liq.to_string(),
        "chain": "Base (8453)"
    }))?);
    Ok(())
}
