use anyhow::Context;
use serde_json::{json, Value};

use crate::config;
use crate::onchainos;
use crate::rpc;

/// Fetch and display the health factor and account summary for a SparkLend position.
pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<Value> {
    let user_addr = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::wallet_address(chain_id).context(
            "No --from address specified and could not resolve active wallet.",
        )?
    };

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(config::POOL_ADDRESSES_PROVIDER, config::RPC_URL)
        .await
        .context("Failed to resolve SparkLend Pool address")?;

    let data = rpc::get_user_account_data(&pool_addr, &user_addr, config::RPC_URL)
        .await
        .context("Failed to fetch user account data")?;

    let hf_display = if data.health_factor >= u128::MAX / 2 {
        "no_debt".to_string()
    } else {
        format!("{:.2}", data.health_factor_f64())
    };
    let status = if data.health_factor >= u128::MAX / 2 {
        "no_debt"
    } else {
        data.health_factor_status()
    };

    let liq_threshold_pct = data.current_liquidation_threshold as f64 / 100.0;
    let ltv_pct = data.ltv as f64 / 100.0;

    Ok(json!({
        "ok": true,
        "chain": config::CHAIN_NAME,
        "chainId": chain_id,
        "userAddress": user_addr,
        "poolAddress": pool_addr,
        "healthFactor": hf_display,
        "healthFactorStatus": status,
        "totalCollateralUSD": format!("{:.2}", data.total_collateral_usd()),
        "totalDebtUSD": format!("{:.2}", data.total_debt_usd()),
        "availableBorrowsUSD": format!("{:.2}", data.available_borrows_usd()),
        "currentLiquidationThreshold": format!("{:.2}%", liq_threshold_pct),
        "loanToValue": format!("{:.2}%", ltv_pct),
        "raw": {
            "healthFactorRaw": data.health_factor.to_string(),
            "totalCollateralBase": data.total_collateral_base.to_string(),
            "totalDebtBase": data.total_debt_base.to_string(),
            "availableBorrowsBase": data.available_borrows_base.to_string()
        }
    }))
}
