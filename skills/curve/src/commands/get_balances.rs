// commands/get_balances.rs — Query user LP token balances across Curve pools
use crate::{api, config, onchainos, rpc};
use anyhow::Result;

pub async fn run(chain_id: u64, wallet: Option<String>) -> Result<()> {
    let chain_name = config::chain_name(chain_id);
    let rpc_url = config::rpc_url(chain_id);

    // Resolve wallet address
    let wallet_addr = match wallet {
        Some(w) => w,
        None => {
            let w = onchainos::resolve_wallet(chain_id)?;
            if w.is_empty() {
                anyhow::bail!("Cannot determine wallet address. Pass --wallet or ensure onchainos is logged in.");
            }
            w
        }
    };

    // Fetch all pools
    let pools = api::get_all_pools(chain_name).await?;

    // Check LP token balance for each pool (LP token = pool address in Curve)
    let mut positions = Vec::new();
    for pool in &pools {
        let balance = rpc::balance_of(&pool.address, &wallet_addr, rpc_url)
            .await
            .unwrap_or(0);
        if balance > 0 {
            let coins: Vec<_> = pool
                .coins
                .iter()
                .map(|c| c.symbol.as_str())
                .collect();
            positions.push(serde_json::json!({
                "pool_id": pool.id,
                "pool_name": pool.name,
                "pool_address": pool.address,
                "coins": coins,
                "lp_balance_raw": balance.to_string(),
                "tvl_usd": pool.usd_total
            }));
        }
    }

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "wallet": wallet_addr,
            "chain": chain_name,
            "positions_count": positions.len(),
            "positions": positions
        })
    );
    Ok(())
}
