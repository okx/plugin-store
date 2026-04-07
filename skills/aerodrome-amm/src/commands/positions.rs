use clap::Args;
use crate::config::{factory_address, resolve_token_address, rpc_url};
use crate::onchainos::resolve_wallet;
use crate::rpc::{factory_get_pool, get_balance, pool_get_reserves, pool_token0, pool_token1, pool_total_supply};

const CHAIN_ID: u64 = 8453;

/// Common token pairs to check for LP positions (Base mainnet)
const COMMON_PAIRS: &[(&str, &str, bool)] = &[
    ("0x4200000000000000000000000000000000000006", "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", false), // WETH/USDC volatile
    ("0x4200000000000000000000000000000000000006", "0x940181a94A35A4569E4529A3CDfB74e38FD98631", false), // WETH/AERO volatile
    ("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb", true),  // USDC/DAI stable
    ("0x4200000000000000000000000000000000000006", "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf", false), // WETH/cbBTC volatile
    ("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2", true), // USDC/USDT stable
];

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub owner: Option<String>,
    /// Specific pool address to check LP balance for
    #[arg(long)]
    pub pool: Option<String>,
    /// Token A to look up specific pool (requires --token-b and optionally --stable)
    #[arg(long)]
    pub token_a: Option<String>,
    /// Token B to look up specific pool
    #[arg(long)]
    pub token_b: Option<String>,
    /// Pool type for lookup (volatile=false, stable=true)
    #[arg(long)]
    pub stable: Option<bool>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let factory = factory_address();

    let owner = match args.owner {
        Some(addr) => addr,
        None => resolve_wallet(CHAIN_ID)?,
    };

    println!("Fetching Aerodrome AMM LP positions for wallet: {}", owner);

    let mut positions = Vec::new();

    // --- Case 1: Specific pool address ---
    if let Some(pool_addr) = args.pool {
        let lp_bal = get_balance(&pool_addr, &owner, rpc).await?;
        if lp_bal > 0 {
            let token0 = pool_token0(&pool_addr, rpc).await?;
            let token1 = pool_token1(&pool_addr, rpc).await?;
            let (reserve0, reserve1) = pool_get_reserves(&pool_addr, rpc).await?;
            let total_supply = pool_total_supply(&pool_addr, rpc).await?;
            positions.push(build_position_json(&pool_addr, &token0, &token1, lp_bal, reserve0, reserve1, total_supply));
        }
    }
    // --- Case 2: Specific token pair ---
    else if args.token_a.is_some() && args.token_b.is_some() {
        let token_a = resolve_token_address(&args.token_a.unwrap());
        let token_b = resolve_token_address(&args.token_b.unwrap());
        let stable_options: Vec<bool> = match args.stable {
            Some(s) => vec![s],
            None => vec![false, true],
        };
        for stable in stable_options {
            let pool_addr = factory_get_pool(&token_a, &token_b, stable, factory, rpc).await?;
            if pool_addr == "0x0000000000000000000000000000000000000000" {
                continue;
            }
            let lp_bal = get_balance(&pool_addr, &owner, rpc).await?;
            if lp_bal > 0 {
                let (reserve0, reserve1) = pool_get_reserves(&pool_addr, rpc).await?;
                let total_supply = pool_total_supply(&pool_addr, rpc).await?;
                positions.push(build_position_json(&pool_addr, &token_a, &token_b, lp_bal, reserve0, reserve1, total_supply));
            }
        }
    }
    // --- Case 3: Scan common pairs ---
    else {
        for (ta, tb, stable) in COMMON_PAIRS {
            let pool_addr = factory_get_pool(ta, tb, *stable, factory, rpc).await?;
            if pool_addr == "0x0000000000000000000000000000000000000000" {
                continue;
            }
            let lp_bal = get_balance(&pool_addr, &owner, rpc).await?;
            if lp_bal > 0 {
                let (reserve0, reserve1) = pool_get_reserves(&pool_addr, rpc).await?;
                let total_supply = pool_total_supply(&pool_addr, rpc).await?;
                positions.push(build_position_json(&pool_addr, ta, tb, lp_bal, reserve0, reserve1, total_supply));
                println!(
                    "  Found: pool={} token0={} token1={} stable={} lpBalance={}",
                    pool_addr, ta, tb, stable, lp_bal
                );
            }
        }
    }

    println!(
        "{{\"ok\":true,\"owner\":\"{}\",\"positions\":{}}}",
        owner,
        serde_json::to_string(&positions)?
    );

    Ok(())
}

fn build_position_json(
    pool: &str,
    token0: &str,
    token1: &str,
    lp_balance: u128,
    reserve0: u128,
    reserve1: u128,
    total_supply: u128,
) -> serde_json::Value {
    // Calculate share of pool
    let share = if total_supply > 0 {
        (lp_balance as f64 / total_supply as f64) * 100.0
    } else {
        0.0
    };
    // Estimated tokens based on share
    let token0_amount = if total_supply > 0 {
        (lp_balance as u128) * reserve0 / total_supply
    } else {
        0
    };
    let token1_amount = if total_supply > 0 {
        (lp_balance as u128) * reserve1 / total_supply
    } else {
        0
    };

    serde_json::json!({
        "pool": pool,
        "token0": token0,
        "token1": token1,
        "lpBalance": lp_balance.to_string(),
        "poolSharePct": format!("{:.6}", share),
        "estimatedToken0": token0_amount.to_string(),
        "estimatedToken1": token1_amount.to_string(),
        "totalSupply": total_supply.to_string(),
    })
}
