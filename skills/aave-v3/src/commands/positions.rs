use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// View current Aave V3 positions.
///
/// Flow:
/// 1. Call onchainos defi positions for the chain
/// 2. Enrich with health factor from Pool.getUserAccountData
/// 3. Return combined view
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

    // Step 1: get positions from onchainos
    let positions_result = onchainos::defi_positions(chain_id, &user_addr);
    let positions = match positions_result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Warning: onchainos defi positions failed: {}. Continuing with health factor only.", e);
            json!(null)
        }
    };

    // Step 2: enrich with health factor
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

    let account_data = rpc::get_user_account_data(&pool_addr, &user_addr, cfg.rpc_url)
        .await
        .context("Failed to fetch user account data")?;

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "userAddress": user_addr,
        "healthFactor": format!("{:.2}", account_data.health_factor_f64()),
        "healthFactorStatus": account_data.health_factor_status(),
        "totalCollateralUSD": format!("{:.2}", account_data.total_collateral_usd()),
        "totalDebtUSD": format!("{:.2}", account_data.total_debt_usd()),
        "availableBorrowsUSD": format!("{:.2}", account_data.available_borrows_usd()),
        "positions": positions
    }))
}
