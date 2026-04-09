use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config::{get_chain_config, HF_WARN_THRESHOLD};
use crate::onchainos;
use crate::rpc;

/// Enable or disable an asset as collateral via Pool.setUserUseReserveAsCollateral().
///
/// Flow:
/// 1. Resolve Pool address at runtime
/// 2. Check current health factor — warn if disabling collateral would risk liquidation
/// 3. Encode calldata and submit
pub async fn run(
    chain_id: u64,
    asset: &str,
    enable: bool,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let from_addr = resolve_from(from, chain_id)?;

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

    // Pre-flight: check health factor
    let account_data = rpc::get_user_account_data(&pool_addr, &from_addr, cfg.rpc_url)
        .await
        .context("Failed to fetch user account data")?;

    let hf = account_data.health_factor_f64();
    let mut warnings: Vec<String> = vec![];

    if !enable && hf < HF_WARN_THRESHOLD && account_data.total_debt_base > 0 {
        warnings.push(format!(
            "WARNING: Disabling collateral when health factor is {:.2} (below {}) may trigger liquidation. Proceed with caution.",
            hf, HF_WARN_THRESHOLD
        ));
    }

    // Encode calldata
    let calldata = calldata::encode_set_collateral(asset, enable)
        .context("Failed to encode setUserUseReserveAsCollateral calldata")?;

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
        "useAsCollateral": enable,
        "poolAddress": pool_addr,
        "healthFactorBefore": format!("{:.4}", hf),
        "warnings": warnings,
        "dryRun": dry_run,
        "raw": result
    }))
}

fn resolve_from(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    onchainos::wallet_address(chain_id).context("No --from address and could not resolve active wallet.")
}
