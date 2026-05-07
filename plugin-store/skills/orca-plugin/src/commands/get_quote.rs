use crate::api;
use crate::config::{DEFAULT_SLIPPAGE_BPS, PRICE_IMPACT_WARN_THRESHOLD, SOL_NATIVE_MINT, WSOL_MINT};
use clap::Args;
use serde::Serialize;

#[derive(Args, Debug)]
pub struct GetQuoteArgs {
    /// Input token mint address (use native SOL: 11111111111111111111111111111111)
    #[arg(long)]
    pub from_token: String,

    /// Output token mint address
    #[arg(long)]
    pub to_token: String,

    /// Amount in human-readable units (e.g. 0.5 for 0.5 SOL, 10 for 10 USDC)
    #[arg(long)]
    pub amount: f64,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u64,

    /// Pool address to quote against (optional — uses best TVL pool if omitted)
    #[arg(long)]
    pub pool: Option<String>,
}

#[derive(Serialize)]
struct QuoteResult {
    ok: bool,
    from_token: String,
    from_token_symbol: String,
    to_token: String,
    to_token_symbol: String,
    amount_in: f64,
    estimated_amount_out: f64,
    minimum_amount_out: f64,
    slippage_bps: u64,
    slippage_pct: f64,
    fee_rate_pct: f64,
    price: f64,
    pool_address: String,
    pool_tvl_usd: f64,
    estimated_price_impact_pct: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

pub async fn execute(args: &GetQuoteArgs) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let all_pools = api::fetch_all_pools(&client).await?;

    // Normalize native SOL to wSOL for pool lookup
    let normalize = |mint: &str| -> String {
        if mint == SOL_NATIVE_MINT {
            WSOL_MINT.to_string()
        } else {
            mint.to_string()
        }
    };
    let from_norm = normalize(&args.from_token);
    let to_norm = normalize(&args.to_token);

    let mut matching = api::filter_pools_by_pair(&all_pools, &from_norm, &to_norm);
    if matching.is_empty() {
        anyhow::bail!(
            "No Orca pools found for pair {} / {}",
            args.from_token,
            args.to_token
        );
    }

    // Sort by TVL descending, use best pool (or user-specified pool)
    matching.sort_by(|a, b| {
        b.tvl
            .unwrap_or(0.0)
            .partial_cmp(&a.tvl.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let pool = if let Some(ref pool_addr) = args.pool {
        matching
            .iter()
            .find(|p| &p.address == pool_addr)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Specified pool {} not found in matching pools", pool_addr))?
    } else {
        matching[0]
    };

    // Determine direction: is from_token == tokenA of the pool?
    let token_a_is_from = pool.token_a.mint == from_norm;
    let (from_sym, to_sym, from_decimals, to_decimals) = if token_a_is_from {
        (
            pool.token_a.symbol.clone(),
            pool.token_b.symbol.clone(),
            pool.token_a.decimals as u32,
            pool.token_b.decimals as u32,
        )
    } else {
        (
            pool.token_b.symbol.clone(),
            pool.token_a.symbol.clone(),
            pool.token_b.decimals as u32,
            pool.token_a.decimals as u32,
        )
    };

    // Pool price is always: tokenB per tokenA
    // If selling tokenA: out = amount_in * price
    // If selling tokenB: out = amount_in / price
    let price = pool.price.unwrap_or(0.0);
    if price <= 0.0 {
        anyhow::bail!("Pool price is zero or unavailable");
    }

    // Convert to raw units for display (just use human-readable math here)
    let estimated_out = if token_a_is_from {
        // selling tokenA, getting tokenB
        // price = tokenB per tokenA (normalized by decimals)
        args.amount * price
    } else {
        // selling tokenB, getting tokenA
        // price = tokenB per tokenA, so tokenA = tokenB / price
        args.amount / price
    };

    let slippage_multiplier = 1.0 - (args.slippage_bps as f64 / 10_000.0);
    let minimum_out = estimated_out * slippage_multiplier;
    let fee_rate = pool.lp_fee_rate.unwrap_or(0.0);

    // Estimate price impact: amount_in_usd / pool_tvl * 200 (2x conservative for CLMM)
    let tvl = pool.tvl.unwrap_or(1_000_000.0);
    // Rough USD value of input (use price for tokenA/SOL equivalent)
    let price_impact = api::estimate_price_impact(args.amount * price.max(1.0), tvl);

    let warning = if price_impact >= PRICE_IMPACT_WARN_THRESHOLD {
        Some(format!(
            "Estimated price impact {:.2}% exceeds warning threshold",
            price_impact
        ))
    } else {
        None
    };

    let quote = QuoteResult {
        ok: true,
        from_token: args.from_token.clone(),
        from_token_symbol: from_sym,
        to_token: args.to_token.clone(),
        to_token_symbol: to_sym,
        amount_in: args.amount,
        estimated_amount_out: estimated_out,
        minimum_amount_out: minimum_out,
        slippage_bps: args.slippage_bps,
        slippage_pct: args.slippage_bps as f64 / 100.0,
        fee_rate_pct: fee_rate * 100.0,
        price,
        pool_address: pool.address.clone(),
        pool_tvl_usd: tvl,
        estimated_price_impact_pct: price_impact,
        warning,
    };

    // Suppress unused import warning for decimals vars
    let _ = (from_decimals, to_decimals);

    println!("{}", serde_json::to_string_pretty(&quote)?);
    Ok(())
}
