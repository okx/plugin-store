// commands/get_pool_info.rs — Query single Curve pool details
use crate::{api, config, rpc};
use anyhow::Result;

pub async fn run(chain_id: u64, pool_address: String) -> Result<()> {
    let chain_name = config::chain_name(chain_id);
    let rpc_url = config::rpc_url(chain_id);

    // Fetch all pools from API to find this pool
    let pools = api::get_all_pools(chain_name).await?;
    let pool = api::find_pool_by_address(&pools, &pool_address);

    // Also fetch on-chain data
    let virtual_price_hex = rpc::eth_call(
        &pool_address,
        "0xbb7b8b80", // virtual_price()
        rpc_url,
    )
    .await
    .unwrap_or_default();

    let virtual_price = rpc::decode_uint128(&virtual_price_hex);

    let fee_hex = rpc::eth_call(
        &pool_address,
        "0xddca3f43", // fee()
        rpc_url,
    )
    .await
    .unwrap_or_default();
    let fee_raw = rpc::decode_uint128(&fee_hex);
    // Curve fee is in 1e10 units (1e10 = 100%, 4000000 = 0.04%)
    let fee_pct = fee_raw as f64 / 1e10 * 100.0;

    if let Some(p) = pool {
        let coins: Vec<_> = p
            .coins
            .iter()
            .enumerate()
            .map(|(i, c)| {
                serde_json::json!({
                    "index": i,
                    "symbol": c.symbol,
                    "address": c.address,
                    "decimals": c.decimals
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "pool": {
                    "id": p.id,
                    "name": p.name,
                    "address": p.address,
                    "coins": coins,
                    "tvl_usd": p.usd_total,
                    "virtual_price_raw": virtual_price.to_string(),
                    "fee_pct": format!("{:.4}%", fee_pct),
                    "fee_raw": fee_raw.to_string()
                }
            })
        );
    } else {
        // Pool not found in API registry
        anyhow::bail!(
            "Pool {} not found in the Curve API registry for this chain. \
            Use `get-pools` to list available pools.",
            pool_address
        );
    }
    Ok(())
}
