use crate::config::get_chain_config;
use crate::rpc;

/// List Euler V2 lending markets (EVaults) on the given chain.
/// Queries eVaultFactory for first 20 vaults, then fetches live data per vault.
pub async fn run(chain_id: u64, asset_filter: Option<&str>) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    // How many vaults exist?
    let count_hex = rpc::eth_call(cfg.evault_factory, "0x0a68b7ba", rpc).await?;
    let total = rpc::parse_u128_from_hex(&count_hex).unwrap_or(0) as u64;

    // Fetch first 20 (or fewer)
    let fetch_end = total.min(20);
    let vaults = rpc::factory_get_vaults(cfg.evault_factory, 0, fetch_end, rpc).await?;

    let mut markets = Vec::new();
    let filter_upper = asset_filter.map(|s| s.to_uppercase());

    for vault_addr in &vaults {
        // asset()
        let asset_addr = match rpc::vault_asset(vault_addr, rpc).await {
            Ok(a) => a,
            Err(_) => continue,
        };

        // underlying symbol
        let asset_symbol = rpc::erc20_symbol(&asset_addr, rpc).await.unwrap_or_else(|_| "UNKNOWN".to_string());

        // Apply filter
        if let Some(ref f) = filter_upper {
            if !asset_symbol.to_uppercase().contains(f.as_str())
                && !vault_addr.to_lowercase().contains(&f.to_lowercase())
            {
                continue;
            }
        }

        let decimals = rpc::erc20_decimals(&asset_addr, rpc).await.unwrap_or(18);
        let total_assets = rpc::vault_total_assets(vault_addr, rpc).await.unwrap_or(0);
        let tvl_human = rpc::format_amount(total_assets, decimals);

        // borrow rate (per-second ray)
        let borrow_rate_ray = rpc::vault_interest_rate(vault_addr, rpc).await.unwrap_or(0);
        let borrow_apr = rpc::ray_to_apr_pct(borrow_rate_ray);

        markets.push(serde_json::json!({
            "vault": vault_addr,
            "asset": asset_addr,
            "assetSymbol": asset_symbol,
            "decimals": decimals,
            "totalAssets": total_assets.to_string(),
            "tvl": tvl_human,
            "borrowAprPct": format!("{:.2}", borrow_apr),
            "supplyInstruction": format!(
                "euler-v2 --chain {} supply --vault {} --amount <n>",
                chain_id, vault_addr
            )
        }));
    }

    let output = serde_json::json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "eVaultFactory": cfg.evault_factory,
        "totalVaults": total,
        "showing": markets.len(),
        "markets": markets,
        "note": "borrowAprPct = borrow APR in %. Supply via 'euler-v2 --chain <id> supply --vault <addr> --amount <n>'"
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
