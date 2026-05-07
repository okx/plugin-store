use crate::api;
use crate::calldata;
use crate::config::{get_chain_config, chain_name};
use crate::onchainos;
use crate::rpc;

/// View user's Morpho Blue and MetaMorpho vault positions with health factors.
pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let user_string = onchainos::resolve_wallet(from, chain_id).await?;
    let user = user_string.as_str();

    // Fetch Morpho Blue positions
    let market_positions = api::get_user_positions(user, chain_id).await?;
    // Fetch MetaMorpho vault positions
    let vault_positions = api::get_vault_positions(user, chain_id).await?;

    let mut positions_out = Vec::new();
    for pos in &market_positions {
        let loan_symbol = pos.market.loan_asset
            .as_ref()
            .map(|a| a.symbol.clone())
            .unwrap_or_default();
        let collateral_symbol = pos.market.collateral_asset
            .as_ref()
            .map(|a| a.symbol.clone())
            .unwrap_or_default();
        let loan_decimals = pos.market.loan_asset
            .as_ref()
            .and_then(|a| a.decimals)
            .unwrap_or(18);
        let coll_decimals = pos.market.collateral_asset
            .as_ref()
            .and_then(|a| a.decimals)
            .unwrap_or(18);

        let borrow_assets_raw: u128 = pos.state.borrow_assets.as_deref().unwrap_or("0").parse().unwrap_or(0);
        let supply_assets_raw: u128 = pos.state.supply_assets.as_deref().unwrap_or("0").parse().unwrap_or(0);
        let collateral_raw: u128 = pos.state.collateral.as_deref().unwrap_or("0").parse().unwrap_or(0);

        positions_out.push(serde_json::json!({
            "marketId": pos.market.unique_key,
            "loanAsset": loan_symbol,
            "collateralAsset": collateral_symbol,
            "supplyAssets": calldata::format_amount(supply_assets_raw, loan_decimals),
            "borrowAssets": calldata::format_amount(borrow_assets_raw, loan_decimals),
            "collateral": calldata::format_amount(collateral_raw, coll_decimals),
        }));
    }

    let mut vaults_out = Vec::new();
    for pos in &vault_positions {
        let asset_symbol = pos.vault.asset.as_ref().map(|a| a.symbol.clone()).unwrap_or_default();
        let asset_decimals = pos.vault.asset.as_ref().and_then(|a| a.decimals).unwrap_or(18);
        let assets_raw: u128 = pos.assets.as_deref().unwrap_or("0").parse().unwrap_or(0);
        let apy = pos.vault.state.as_ref().and_then(|s| s.apy).unwrap_or(0.0);

        vaults_out.push(serde_json::json!({
            "vaultAddress": pos.vault.address,
            "vaultName": pos.vault.name,
            "asset": asset_symbol,
            "balance": calldata::format_amount(assets_raw, asset_decimals),
            "apy": format!("{:.4}%", apy * 100.0),
        }));
    }

    let output = serde_json::json!({
        "ok": true,
        "user": user,
        "chain": chain_name(chain_id),
        "chainId": chain_id,
        "bluePositions": positions_out,
        "vaultPositions": vaults_out,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

