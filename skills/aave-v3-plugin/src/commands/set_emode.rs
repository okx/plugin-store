use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Set E-Mode category via Pool.setUserEMode().
///
/// E-Mode categories:
///   0 = No E-Mode (default)
///   1 = Stablecoins (higher LTV for correlated stablecoin assets)
///   2 = ETH-correlated assets (chain-specific)
///
/// Flow:
/// 1. Resolve Pool address at runtime
/// 2. Encode setUserEMode calldata
/// 3. Submit via onchainos wallet contract-call
pub async fn run(
    chain_id: u64,
    category: u8,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let from_addr = resolve_from(from, chain_id)?;

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

    // Encode calldata
    let calldata = calldata::encode_set_emode(category)
        .context("Failed to encode setUserEMode calldata")?;

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

    let category_name = match category {
        0 => "No E-Mode",
        1 => "Stablecoins",
        2 => "ETH-correlated",
        _ => "Unknown",
    };

    Ok(json!({
        "ok": true,
        "txHash": tx_hash,
        "categoryId": category,
        "categoryName": category_name,
        "poolAddress": pool_addr,
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
