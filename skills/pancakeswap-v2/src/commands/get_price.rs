use crate::config::{resolve_token_address, FACTORY_V2, BSC_RPC, CHAIN_ID};
use crate::rpc::{factory_get_pair, get_reserves, get_token0};

/// Get the price of tokenA in terms of tokenB, derived from on-chain reserves.
/// Price = reserveB / reserveA (assumes both tokens have the same decimals, e.g. 18).
/// For tokens with different decimals, the caller should adjust externally.
pub async fn run(token_a: &str, token_b: &str) -> anyhow::Result<()> {
    let chain_id = CHAIN_ID;
    let rpc = BSC_RPC;

    let addr_a = resolve_token_address(token_a, chain_id);
    let addr_b = resolve_token_address(token_b, chain_id);

    let pair = factory_get_pair(FACTORY_V2, &addr_a, &addr_b, rpc).await?;
    if pair == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("Pair does not exist for {} / {}", token_a, token_b);
    }

    let (r0, r1) = get_reserves(&pair, rpc).await?;
    let token0 = get_token0(&pair, rpc).await?;

    // Determine ordering
    let (reserve_a, reserve_b) = if token0.to_lowercase() == addr_a.to_lowercase() {
        (r0, r1)
    } else {
        (r1, r0)
    };

    if reserve_a == 0 {
        anyhow::bail!("Reserve for {} is zero — pool may be empty", token_a);
    }

    // Price as floating point with 18-decimal precision
    // Both BSC tokens typically have 18 decimals (including BSC-USD/USDT)
    // price = reserveB / reserveA (as f64 for display)
    let price = reserve_b as f64 / reserve_a as f64;

    println!("PancakeSwap V2 Price");
    println!("  pair:    {}", pair);
    println!("  {} reserve: {}", token_a.to_uppercase(), reserve_a);
    println!("  {} reserve: {}", token_b.to_uppercase(), reserve_b);
    println!("  1 {} = {:.6} {} (from reserves, assumes equal decimals)", token_a.to_uppercase(), price, token_b.to_uppercase());

    Ok(())
}
