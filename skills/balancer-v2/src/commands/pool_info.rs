/// pool-info command — get detailed info for a specific pool

use anyhow::Result;
use serde::Serialize;

use crate::config;
use crate::rpc;

#[derive(Debug, Serialize)]
struct TokenInfo {
    address: String,
    balance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight_pct: Option<String>,
}

#[derive(Debug, Serialize)]
struct PoolDetails {
    pool_id: String,
    pool_address: String,
    specialization: u8,
    swap_fee_pct: String,
    total_supply: String,
    tokens: Vec<TokenInfo>,
    chain_id: u64,
}

pub async fn run(pool_id: &str, chain_id: u64) -> Result<()> {
    let rpc_url = config::rpc_url(chain_id);
    let vault = config::VAULT_ADDRESS;

    // getPool
    let (pool_addr, specialization) = rpc::get_pool(pool_id, vault, rpc_url).await?;

    // getPoolTokens
    let (tokens, balances, _last_change) =
        rpc::get_pool_tokens(pool_id, vault, rpc_url).await?;

    // getSwapFeePercentage
    let swap_fee = rpc::get_swap_fee(&pool_addr, rpc_url).await.unwrap_or(0);
    let swap_fee_pct = format!("{:.4}", swap_fee as f64 / 1e18 * 100.0);

    // totalSupply (BPT)
    let total_supply = rpc::get_total_supply(&pool_addr, rpc_url).await.unwrap_or(0);

    // getNormalizedWeights (WeightedPool only — may fail for stable pools)
    let weights = rpc::get_normalized_weights(&pool_addr, rpc_url).await.unwrap_or_default();

    let token_infos: Vec<TokenInfo> = tokens
        .iter()
        .enumerate()
        .map(|(i, addr)| {
            let balance = balances.get(i).copied().unwrap_or(0);
            let weight_pct = weights.get(i).map(|w| format!("{:.2}", *w as f64 / 1e18 * 100.0));
            TokenInfo {
                address: addr.clone(),
                balance: balance.to_string(),
                weight_pct,
            }
        })
        .collect();

    let details = PoolDetails {
        pool_id: pool_id.to_string(),
        pool_address: pool_addr,
        specialization,
        swap_fee_pct,
        total_supply: total_supply.to_string(),
        tokens: token_infos,
        chain_id,
    };

    println!("{}", serde_json::to_string_pretty(&details)?);
    Ok(())
}
