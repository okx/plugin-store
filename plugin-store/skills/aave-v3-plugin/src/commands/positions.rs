use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// View current Aave V3 positions.
///
/// Data sources:
/// - on-chain Pool.getUserAccountData: aggregate health factor, LTV, liquidation threshold
/// - onchainos defi position-detail (platform 10): per-asset SUPPLY / BORROW breakdown
pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve user address
    let user_addr = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::wallet_address(chain_id).context(
            "No --from address specified and could not resolve active wallet.",
        )?
    };

    // Resolve Pool address at runtime (never hardcoded)
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

    // Fetch aggregate account data on-chain via Pool.getUserAccountData
    let account_data = rpc::get_user_account_data(&pool_addr, &user_addr, cfg.rpc_url)
        .await
        .context("Failed to fetch user account data from on-chain Aave Pool")?;

    // Fetch per-asset SUPPLY / BORROW breakdown via onchainos
    let per_asset = fetch_per_asset_positions(chain_id, &user_addr);

    // When a wallet has no Aave position, the contract returns uint256.max as the health factor.
    // Detect this sentinel and replace with a human-readable label instead of a huge number.
    let hf_display = if account_data.health_factor >= u128::MAX / 2 {
        "no_debt".to_string()
    } else {
        format!("{:.4}", account_data.health_factor_f64())
    };
    let hf_status = if account_data.health_factor >= u128::MAX / 2 {
        "no_debt"
    } else {
        account_data.health_factor_status()
    };

    // When there is no collateral, LTV and liquidation threshold are category defaults
    // returned by Aave even for empty positions — zero them out to avoid confusion.
    let (liq_threshold_display, ltv_display) = if account_data.total_collateral_base == 0 {
        ("0.00%".to_string(), "0.00%".to_string())
    } else {
        (
            format!("{:.2}%", account_data.current_liquidation_threshold as f64 / 100.0),
            format!("{:.2}%", account_data.ltv as f64 / 100.0),
        )
    };

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "userAddress": user_addr,
        "poolAddress": pool_addr,
        "healthFactor": hf_display,
        "healthFactorStatus": hf_status,
        "totalCollateralUSD": format!("{:.2}", account_data.total_collateral_usd()),
        "totalDebtUSD": format!("{:.2}", account_data.total_debt_usd()),
        "availableBorrowsUSD": format!("{:.2}", account_data.available_borrows_usd()),
        "currentLiquidationThreshold": liq_threshold_display,
        "loanToValue": ltv_display,
        "positions": per_asset
    }))
}

/// Parse onchainos defi position-detail response into clean SUPPLY/BORROW lists.
/// Returns null if no Aave positions found or onchainos call fails.
fn fetch_per_asset_positions(chain_id: u64, user_addr: &str) -> Value {
    let raw = match onchainos::defi_position_detail(chain_id, user_addr) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[positions] onchainos position-detail failed: {}", e);
            return json!(null);
        }
    };

    let chain_idx = chain_id.to_string();
    let empty = vec![];

    // Navigate: data[0].walletIdPlatformDetailList[0].networkHoldVoList
    let networks = raw["data"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|p| p["walletIdPlatformDetailList"].as_array())
        .and_then(|a| a.first())
        .and_then(|w| w["networkHoldVoList"].as_array())
        .unwrap_or(&empty);

    // Find the network entry matching this chain
    let network = networks.iter().find(|n| {
        n["chainIndex"].as_str() == Some(&chain_idx)
    });

    let Some(network) = network else {
        return json!({"supply": [], "borrow": []});
    };

    let markets = network["investMarketTokenBalanceVoList"]
        .as_array()
        .unwrap_or(&empty);

    let mut supply: Vec<Value> = Vec::new();
    let mut borrow: Vec<Value> = Vec::new();

    for market in markets {
        let asset_map = &market["assetMap"];

        if let Some(supply_list) = asset_map["SUPPLY"].as_array() {
            for item in supply_list {
                let token = item["assetsTokenList"].as_array()
                    .and_then(|a| a.first())
                    .cloned()
                    .unwrap_or(json!({}));
                supply.push(json!({
                    "asset": item["investmentName"].as_str().unwrap_or("?"),
                    "tokenAddress": token["tokenAddress"].as_str().unwrap_or("?"),
                    "amount": token["coinAmount"].as_str().unwrap_or("0"),
                    "valueUSD": item["totalValue"].as_str().unwrap_or("0"),
                    "marketId": item["marketId"].as_str().unwrap_or("?")
                }));
            }
        }

        if let Some(borrow_list) = asset_map["BORROW"].as_array() {
            for item in borrow_list {
                let token = item["assetsTokenList"].as_array()
                    .and_then(|a| a.first())
                    .cloned()
                    .unwrap_or(json!({}));
                // totalValue is negative for borrows; strip the sign for amount display
                let value_str = item["totalValue"].as_str().unwrap_or("0");
                let value_abs = value_str.trim_start_matches('-');
                borrow.push(json!({
                    "asset": item["investmentName"].as_str().unwrap_or("?"),
                    "tokenAddress": token["tokenAddress"].as_str().unwrap_or("?"),
                    "amount": token["coinAmount"].as_str().unwrap_or("0"),
                    "valueUSD": value_abs,
                    "marketId": item["marketId"].as_str().unwrap_or("?")
                }));
            }
        }
    }

    json!({"supply": supply, "borrow": borrow})
}
