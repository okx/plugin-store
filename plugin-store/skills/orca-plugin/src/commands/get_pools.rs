use crate::api::{self, WhirlpoolPool};
use crate::config::DEFAULT_MIN_POOL_TVL_USD;
use clap::Args;
use serde::Serialize;

#[derive(Args, Debug)]
pub struct GetPoolsArgs {
    /// Mint address of the first token (e.g. SOL native mint or SPL mint)
    #[arg(long)]
    pub token_a: String,

    /// Mint address of the second token
    #[arg(long)]
    pub token_b: String,

    /// Minimum TVL in USD to include a pool (default: 10000)
    #[arg(long, default_value_t = DEFAULT_MIN_POOL_TVL_USD)]
    pub min_tvl: f64,

    /// Include pools below min_tvl threshold
    #[arg(long)]
    pub include_low_liquidity: bool,
}

#[derive(Serialize)]
struct PoolResult {
    address: String,
    token_a_mint: String,
    token_a_symbol: String,
    token_b_mint: String,
    token_b_symbol: String,
    tick_spacing: u32,
    fee_rate_pct: f64,
    price: f64,
    tvl_usd: f64,
    volume_24h_usd: f64,
    fee_apr_24h_pct: f64,
    total_apr_24h_pct: f64,
}

#[derive(Serialize)]
struct GetPoolsOutput {
    ok: bool,
    token_a: String,
    token_b: String,
    pools_found: usize,
    pools: Vec<PoolResult>,
}

pub async fn execute(args: &GetPoolsArgs) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let all_pools = api::fetch_all_pools(&client).await?;

    // Normalize token addresses — if user passes native SOL, treat as wSOL for pool matching
    let normalize = |mint: &str| -> String {
        if mint == crate::config::SOL_NATIVE_MINT {
            crate::config::WSOL_MINT.to_string()
        } else {
            mint.to_string()
        }
    };
    let token_a = normalize(&args.token_a);
    let token_b = normalize(&args.token_b);

    let mut matching: Vec<&WhirlpoolPool> = api::filter_pools_by_pair(&all_pools, &token_a, &token_b);

    // Sort by TVL descending
    matching.sort_by(|a, b| {
        b.tvl
            .unwrap_or(0.0)
            .partial_cmp(&a.tvl.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Apply TVL filter unless user asked for all
    if !args.include_low_liquidity {
        matching.retain(|p| p.tvl.unwrap_or(0.0) >= args.min_tvl);
    }

    let results: Vec<PoolResult> = matching
        .iter()
        .map(|p| PoolResult {
            address: p.address.clone(),
            token_a_mint: p.token_a.mint.clone(),
            token_a_symbol: p.token_a.symbol.clone(),
            token_b_mint: p.token_b.mint.clone(),
            token_b_symbol: p.token_b.symbol.clone(),
            tick_spacing: p.tick_spacing,
            fee_rate_pct: p.lp_fee_rate.unwrap_or(0.0) * 100.0,
            price: p.price.unwrap_or(0.0),
            tvl_usd: p.tvl.unwrap_or(0.0),
            volume_24h_usd: p.volume.as_ref().and_then(|v| v.day).unwrap_or(0.0),
            fee_apr_24h_pct: p.fee_apr.as_ref().and_then(|a| a.day).unwrap_or(0.0) * 100.0,
            total_apr_24h_pct: p.total_apr.as_ref().and_then(|a| a.day).unwrap_or(0.0) * 100.0,
        })
        .collect();

    let output = GetPoolsOutput {
        ok: true,
        token_a: args.token_a.clone(),
        token_b: args.token_b.clone(),
        pools_found: results.len(),
        pools: results,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
