use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config::{get_chain_config, HF_WARN_THRESHOLD};
use crate::onchainos;
use crate::rpc;

/// Borrow assets from Aave V3 via Pool.borrow() ABI calldata.
///
/// Flow:
/// 1. Resolve from address (active wallet if not specified)
/// 2. Resolve Pool address at runtime via PoolAddressesProvider.getPool()
/// 3. Check availableBorrowsBase and warn if post-borrow HF < 1.1
/// 4. Encode borrow calldata and submit via onchainos wallet contract-call
pub async fn run(
    chain_id: u64,
    asset: &str,
    amount: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve caller address
    let from_addr = resolve_from(from, chain_id)?;

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address from PoolAddressesProvider")?;

    // Pre-flight: check account health
    let account_data = rpc::get_user_account_data(&pool_addr, &from_addr, cfg.rpc_url)
        .await
        .context("Failed to fetch user account data")?;

    let hf = account_data.health_factor_f64();

    // Warn if health factor is already below warning threshold
    let mut warnings: Vec<String> = vec![];
    if hf < HF_WARN_THRESHOLD && account_data.total_debt_base > 0 {
        warnings.push(format!(
            "Current health factor is {:.2} — below the warning threshold of {}. Borrowing more will increase liquidation risk.",
            hf, HF_WARN_THRESHOLD
        ));
    }

    // Check available borrow capacity
    let available_usd = account_data.available_borrows_usd();
    if available_usd <= 0.0 && !dry_run {
        anyhow::bail!(
            "No borrow capacity available. Total collateral: ${:.2}, Total debt: ${:.2}",
            account_data.total_collateral_usd(),
            account_data.total_debt_usd()
        );
    }
    if available_usd <= 0.0 {
        warnings.push(format!(
            "No borrow capacity available (no collateral posted). Total collateral: ${:.2}. \
             This borrow would revert on-chain.",
            account_data.total_collateral_usd()
        ));
    }

    // Note: amount validation is best-effort here since we don't have the USD price
    // of the specific asset. The on-chain tx will revert if over capacity.

    // Resolve asset address and decimals via onchainos token search (handles USDC=6, WBTC=8, etc.)
    let (asset_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;
    let amount_minimal = (amount * 10u128.pow(decimals as u32) as f64) as u128;

    let calldata = calldata::encode_borrow(&asset_addr, amount_minimal, &from_addr)
        .context("Failed to encode borrow calldata")?;

    let result = onchainos::wallet_contract_call(
        chain_id,
        &pool_addr,
        &calldata,
        Some(&from_addr),
        dry_run,
    )
    .context("onchainos wallet contract-call failed")?;

    let tx_hash = result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .or_else(|| result["hash"].as_str())
        .unwrap_or("pending");

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "asset": asset,
        "borrowAmount": amount,
        "borrowAmountMinimal": amount_minimal.to_string(),
        "poolAddress": pool_addr,
        "currentHealthFactor": format!("{:.4}", hf),
        "healthFactorStatus": account_data.health_factor_status(),
        "availableBorrowsUSD": format!("{:.2}", available_usd),
        "warnings": warnings,
        "dryRun": dry_run,
        "raw": result
    }))
}

fn resolve_from(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    onchainos::wallet_address(chain_id).context(
        "No --from address specified and could not resolve active wallet. \
         Run `onchainos wallet status` to check login status.",
    )
}
