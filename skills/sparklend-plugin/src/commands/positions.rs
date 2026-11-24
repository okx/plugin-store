use anyhow::Context;
use serde_json::{json, Value};

use crate::config;
use crate::onchainos;
use crate::rpc;

/// View current SparkLend positions.
///
/// Data source: on-chain Pool.getUserAccountData — aggregate health factor, LTV,
/// liquidation threshold, total collateral/debt/borrow capacity.
pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<Value> {
    let user_addr = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::wallet_address(chain_id).context(
            "No --from address specified and could not resolve active wallet.",
        )?
    };

    // Resolve Pool address at runtime (never hardcoded)
    let pool_addr = rpc::get_pool(config::POOL_ADDRESSES_PROVIDER, config::RPC_URL)
        .await
        .context("Failed to resolve SparkLend Pool address")?;

    // Fetch aggregate account data on-chain via Pool.getUserAccountData
    let account_data = rpc::get_user_account_data(&pool_addr, &user_addr, config::RPC_URL)
        .await
        .context("Failed to fetch user account data from SparkLend Pool")?;

    // When a wallet has no position, the contract returns uint256.max as the health factor.
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

    let (liq_threshold_display, ltv_display) = if account_data.total_collateral_base == 0 {
        ("0.00%".to_string(), "0.00%".to_string())
    } else {
        (
            format!("{:.2}%", account_data.current_liquidation_threshold as f64 / 100.0),
            format!("{:.2}%", account_data.ltv as f64 / 100.0),
        )
    };

    let no_position = account_data.total_collateral_base == 0 && account_data.total_debt_base == 0;

    Ok(json!({
        "ok": true,
        "chain": config::CHAIN_NAME,
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
        "message": if no_position {
            Some("No active SparkLend position. Supply assets to get started.")
        } else {
            None
        }
    }))
}
