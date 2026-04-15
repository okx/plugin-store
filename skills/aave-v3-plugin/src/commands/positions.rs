use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// View current Aave V3 positions.
///
/// Data source: on-chain only via Pool.getUserAccountData (eth_call to public RPC).
/// Returns aggregate totals (totalCollateralUSD, totalDebtUSD, healthFactor).
/// Per-asset supply/borrow breakdown is NOT included — that would require iterating
/// all reserve addresses and calling getUserReserveData for each, which is
/// available via the Aave V3 UI or PoolDataProvider contract directly.
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

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "userAddress": user_addr,
        "poolAddress": pool_addr,
        "healthFactor": format!("{:.4}", account_data.health_factor_f64()),
        "healthFactorStatus": account_data.health_factor_status(),
        "totalCollateralUSD": format!("{:.2}", account_data.total_collateral_usd()),
        "totalDebtUSD": format!("{:.2}", account_data.total_debt_usd()),
        "availableBorrowsUSD": format!("{:.2}", account_data.available_borrows_usd()),
        "currentLiquidationThreshold": format!("{:.2}%", account_data.current_liquidation_threshold as f64 / 100.0),
        "loanToValue": format!("{:.2}%", account_data.ltv as f64 / 100.0),
        "dataSource": "on-chain — Pool.getUserAccountData (aggregate totals only)",
        "note": "Per-asset supply/borrow breakdown requires querying Pool.getUserReserveData for each reserve. Use `aave-v3-plugin reserves` to see available markets."
    }))
}
