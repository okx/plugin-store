use crate::api;
use crate::calldata;
use crate::config::chain_name;

/// List MetaMorpho vaults with APYs, optionally filtered by asset.
pub async fn run(chain_id: u64, asset_filter: Option<&str>) -> anyhow::Result<()> {
    let vaults = api::list_vaults(chain_id, asset_filter).await?;

    let items: Vec<serde_json::Value> = vaults.iter().map(|v| {
        let asset_symbol = v.asset.as_ref().map(|a| a.symbol.as_str()).unwrap_or("?");
        let asset_addr = v.asset.as_ref().map(|a| a.address.as_str()).unwrap_or("");
        let asset_decimals = v.asset.as_ref().and_then(|a| a.decimals).unwrap_or(18);
        let apy = v.state.as_ref().and_then(|s| s.apy).unwrap_or(0.0);
        let total_assets_raw: u128 = v.state.as_ref()
            .and_then(|s| s.total_assets.as_deref())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        serde_json::json!({
            "address": v.address,
            "name": v.name,
            "symbol": v.symbol,
            "asset": asset_symbol,
            "assetAddress": asset_addr,
            "apy": format!("{:.4}%", apy * 100.0),
            "totalAssets": calldata::format_amount(total_assets_raw, asset_decimals),
        })
    }).collect();

    let output = serde_json::json!({
        "ok": true,
        "chain": chain_name(chain_id),
        "chainId": chain_id,
        "vaultCount": items.len(),
        "vaults": items,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
