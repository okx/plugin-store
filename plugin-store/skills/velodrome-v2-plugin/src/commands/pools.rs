use clap::Args;
use crate::config::{factory_address, resolve_token_address, rpc_url};
use crate::rpc::{factory_get_pool, pool_get_reserves, pool_token0, pool_token1};

#[derive(Args)]
pub struct PoolsArgs {
    /// Token A (symbol or hex address, e.g. WETH, USDC, 0x...)
    #[arg(long)]
    pub token_a: Option<String>,
    /// Token B (symbol or hex address)
    #[arg(long)]
    pub token_b: Option<String>,
    /// Pool type: volatile (false) or stable (true). If omitted, queries both.
    #[arg(long)]
    pub stable: Option<bool>,
    /// Direct pool address to query (alternative to token_a/token_b lookup)
    #[arg(long)]
    pub pool: Option<String>,
}

pub async fn run(args: PoolsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let factory = factory_address();

    // --- Case 1: Direct pool address query ---
    if let Some(pool_addr) = args.pool {
        let token0 = pool_token0(&pool_addr, rpc).await?;
        let token1 = pool_token1(&pool_addr, rpc).await?;
        let (reserve0, reserve1) = pool_get_reserves(&pool_addr, rpc).await?;
        println!(
            "{{\"ok\":true,\"pool\":\"{}\",\"token0\":\"{}\",\"token1\":\"{}\",\"reserve0\":\"{}\",\"reserve1\":\"{}\"}}",
            pool_addr, token0, token1, reserve0, reserve1
        );
        return Ok(());
    }

    // --- Case 2: Token pair lookup ---
    let token_a_raw = args.token_a.clone().unwrap_or_default();
    let token_b_raw = args.token_b.clone().unwrap_or_default();

    if token_a_raw.is_empty() || token_b_raw.is_empty() {
        anyhow::bail!("Provide --token-a and --token-b (or --pool <address>) to query a pool");
    }

    let token_a = resolve_token_address(&token_a_raw);
    let token_b = resolve_token_address(&token_b_raw);

    let stable_options: Vec<bool> = match args.stable {
        Some(s) => vec![s],
        None => vec![false, true],
    };

    let mut pools = Vec::new();

    for stable in stable_options {
        let pool_addr = factory_get_pool(&token_a, &token_b, stable, factory, rpc).await?;
        let deployed = pool_addr != "0x0000000000000000000000000000000000000000";

        if deployed {
            let (reserve0, reserve1) = pool_get_reserves(&pool_addr, rpc).await.unwrap_or((0, 0));
            println!(
                "  stable={}: {} (reserve0={}, reserve1={})",
                stable, pool_addr, reserve0, reserve1
            );
            pools.push(serde_json::json!({
                "stable": stable,
                "address": pool_addr,
                "reserve0": reserve0.to_string(),
                "reserve1": reserve1.to_string(),
                "deployed": true,
            }));
        } else {
            println!("  stable={}: not deployed", stable);
            pools.push(serde_json::json!({
                "stable": stable,
                "address": pool_addr,
                "deployed": false,
            }));
        }
    }

    println!(
        "{{\"ok\":true,\"tokenA\":\"{}\",\"tokenB\":\"{}\",\"pools\":{}}}",
        token_a,
        token_b,
        serde_json::to_string(&pools)?
    );

    Ok(())
}
