// Show APY data for Beefy vaults

use anyhow::Result;
use serde_json::{json, Value};

use crate::api;
use crate::config::chain_id_to_beefy_name;

pub async fn execute(
    chain_id: u64,
    vault_filter: Option<&str>,
    asset_filter: Option<&str>,
    limit: usize,
) -> Result<Value> {
    let chain_name = chain_id_to_beefy_name(chain_id)
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain ID: {}", chain_id))?;

    let (vaults, apy_map) = tokio::try_join!(
        api::fetch_vaults(chain_id),
        api::fetch_apy(),
    )?;

    let asset_lower = asset_filter.map(|s| s.to_lowercase());
    let vault_lower = vault_filter.map(|s| s.to_lowercase());

    // If a specific vault is requested, find it
    if let Some(ref vault_q) = vault_lower {
        if let Some(v) = api::find_vault(&vaults, vault_q) {
            let apy = apy_map.get(&v.id).and_then(|x| *x).unwrap_or(0.0);
            return Ok(json!({
                "ok": true,
                "id": v.id,
                "name": v.name,
                "assets": v.assets,
                "apy": format!("{:.2}%", apy * 100.0),
                "apy_raw": apy,
                "status": v.status,
            }));
        } else {
            anyhow::bail!("Vault not found: {}", vault_q);
        }
    }

    // Otherwise filter and list
    let mut results: Vec<(f64, Value)> = vaults
        .iter()
        .filter(|v| v.is_active())
        .filter(|v| {
            if let Some(ref a) = asset_lower {
                let assets = v.assets.as_deref().unwrap_or(&[]);
                return assets.iter().any(|asset| asset.to_lowercase().contains(a.as_str()));
            }
            true
        })
        .filter_map(|v| {
            let apy = apy_map.get(&v.id).and_then(|x| *x).unwrap_or(0.0);
            // Filter out unrealistically high APYs (> 10000%)
            if apy > 100.0 {
                return None;
            }
            Some((apy, json!({
                "id": v.id,
                "name": v.name,
                "assets": v.assets,
                "platform": v.platform_id,
                "apy": format!("{:.2}%", apy * 100.0),
                "apy_raw": apy,
            })))
        })
        .collect();

    // Sort by APY descending
    results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let entries: Vec<Value> = results.into_iter().take(limit).map(|(_, v)| v).collect();

    Ok(json!({
        "ok": true,
        "chain": chain_name,
        "chain_id": chain_id,
        "count": entries.len(),
        "vaults": entries
    }))
}
