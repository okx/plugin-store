// List Beefy vaults with APY and TVL

use anyhow::Result;
use serde_json::{json, Value};

use crate::api;
use crate::config::chain_id_to_beefy_name;

pub async fn execute(
    chain_id: u64,
    asset_filter: Option<&str>,
    platform_filter: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let chain_name = chain_id_to_beefy_name(chain_id)
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain ID: {}", chain_id))?;

    // Fetch vaults, APY, and TVL in parallel
    let (vaults, apy_map, tvl_map) = tokio::try_join!(
        api::fetch_vaults(chain_id),
        api::fetch_apy(),
        api::fetch_tvl(chain_id),
    )?;

    let asset_lower = asset_filter.map(|s| s.to_lowercase());
    let platform_lower = platform_filter.map(|s| s.to_lowercase());

    let filtered: Vec<Value> = vaults
        .iter()
        .filter(|v| v.is_active())
        .filter(|v| {
            if let Some(ref a) = asset_lower {
                let assets = v.assets.as_deref().unwrap_or(&[]);
                if !assets.iter().any(|asset| asset.to_lowercase().contains(a.as_str())) {
                    return false;
                }
            }
            if let Some(ref p) = platform_lower {
                if v.platform_id.as_deref().map(|x| x.to_lowercase()).as_deref() != Some(p.as_str()) {
                    return false;
                }
            }
            true
        })
        .take(limit)
        .map(|v| {
            let apy = apy_map.get(&v.id).and_then(|x| *x).unwrap_or(0.0);
            let tvl = tvl_map.get(&v.id).copied().unwrap_or(0.0);
            json!({
                "id": v.id,
                "name": v.name,
                "assets": v.assets,
                "token": v.token,
                "earnContractAddress": v.earn_contract_address,
                "tokenAddress": v.token_address,
                "platform": v.platform_id,
                "strategyType": v.strategy_type_id,
                "apy": format!("{:.2}%", apy * 100.0),
                "apy_raw": apy,
                "tvl_usd": format!("${:.2}", tvl),
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "chain": chain_name,
        "chain_id": chain_id,
        "count": filtered.len(),
        "vaults": filtered
    }))
}
