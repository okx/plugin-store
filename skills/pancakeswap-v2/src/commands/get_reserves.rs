use crate::config::{resolve_token_address, FACTORY_V2, BSC_RPC, CHAIN_ID};
use crate::rpc::{factory_get_pair, get_reserves, get_token0};

/// Get reserves for a PancakeSwap V2 pair.
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

    // Determine which reserve is tokenA and which is tokenB
    let (reserve_a, reserve_b) = if token0.to_lowercase() == addr_a.to_lowercase() {
        (r0, r1)
    } else {
        (r1, r0)
    };

    println!("PancakeSwap V2 Reserves");
    println!("  pair:     {}", pair);
    println!("  token0:   {}", token0);
    println!("  {}: {} (raw)", token_a.to_uppercase(), reserve_a);
    println!("  {}: {} (raw)", token_b.to_uppercase(), reserve_b);

    Ok(())
}
