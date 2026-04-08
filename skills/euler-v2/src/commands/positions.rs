use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// View user's Euler V2 supply and borrow positions across known vaults.
pub async fn run(chain_id: u64, from: Option<&str>, dry_run: bool) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id, dry_run)?
    };

    // Query first 20 vaults from factory
    let count_hex = rpc::eth_call(cfg.evault_factory, "0x0a68b7ba", rpc).await?;
    let total = rpc::parse_u128_from_hex(&count_hex).unwrap_or(0) as u64;
    let fetch_end = total.min(20);
    let vaults = rpc::factory_get_vaults(cfg.evault_factory, 0, fetch_end, rpc).await?;

    let mut positions = Vec::new();

    for vault_addr in &vaults {
        let shares = rpc::vault_balance_of(vault_addr, &wallet, rpc).await.unwrap_or(0);
        let debt = rpc::vault_debt_of(vault_addr, &wallet, rpc).await.unwrap_or(0);

        if shares == 0 && debt == 0 {
            continue;
        }

        let asset_addr = rpc::vault_asset(vault_addr, rpc).await.unwrap_or_else(|_| "0x".to_string());
        let asset_symbol = rpc::erc20_symbol(&asset_addr, rpc).await.unwrap_or_else(|_| "UNKNOWN".to_string());
        let decimals = rpc::erc20_decimals(&asset_addr, rpc).await.unwrap_or(18);

        // Convert shares to assets
        let supplied_assets = if shares > 0 {
            rpc::vault_convert_to_assets(vault_addr, shares, rpc).await.unwrap_or(shares)
        } else {
            0
        };

        positions.push(serde_json::json!({
            "vault": vault_addr,
            "asset": asset_addr,
            "assetSymbol": asset_symbol,
            "shares": shares.to_string(),
            "suppliedAssets": supplied_assets.to_string(),
            "supplied": rpc::format_amount(supplied_assets, decimals),
            "debt": debt.to_string(),
            "debtFormatted": rpc::format_amount(debt, decimals),
        }));
    }

    let output = serde_json::json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "wallet": wallet,
        "dryRun": dry_run,
        "positionCount": positions.len(),
        "positions": positions,
        "note": if positions.is_empty() {
            "No active Euler V2 positions found in first 20 vaults."
        } else {
            "Positions shown for first 20 vaults. Use --vault to check a specific vault."
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
