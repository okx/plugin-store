use anyhow::Context;
use serde_json::{json, Value};

use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Supply assets to Aave V3 Pool via direct contract-call.
///
/// Flow:
/// 1. Resolve token contract address (symbol → address via onchainos token search)
/// 2. Resolve Pool address via PoolAddressesProvider
/// 3. Approve token to Pool (ERC-20 approve)
/// 4. Call Pool.supply(asset, amount, onBehalfOf, 0)
pub async fn run(
    chain_id: u64,
    asset: &str,
    amount: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let from_addr = resolve_from(from, chain_id)?;

    // Resolve token address and decimals
    let (token_addr, decimals) = onchainos::resolve_token(asset, chain_id)
        .with_context(|| format!("Could not resolve token address for '{}'", asset))?;

    let amount_minimal = human_to_minimal(amount, decimals as u64);

    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(cfg.pool_addresses_provider, cfg.rpc_url)
        .await
        .context("Failed to resolve Pool address")?;

    if dry_run {
        let approve_calldata = calldata::encode_erc20_approve(&pool_addr, amount_minimal)
            .context("Failed to encode approve calldata")?;
        let supply_calldata = calldata::encode_supply(&token_addr, amount_minimal, &from_addr)
            .context("Failed to encode supply calldata")?;
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, token_addr, approve_calldata, from_addr
        );
        let supply_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, pool_addr, supply_calldata, from_addr
        );
        eprintln!("[dry-run] step 1 approve: {}", approve_cmd);
        eprintln!("[dry-run] step 2 supply: {}", supply_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "asset": asset,
            "tokenAddress": token_addr,
            "amount": amount,
            "amountMinimal": amount_minimal.to_string(),
            "poolAddress": pool_addr,
            "steps": [
                {"step": 1, "action": "approve", "simulatedCommand": approve_cmd},
                {"step": 2, "action": "supply",  "simulatedCommand": supply_cmd}
            ]
        }));
    }

    // Step 1: approve
    let approve_calldata = calldata::encode_erc20_approve(&pool_addr, amount_minimal)
        .context("Failed to encode approve calldata")?;
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        &token_addr,
        &approve_calldata,
        Some(&from_addr),
        false,
    )
    .context("ERC-20 approve failed")?;
    let approve_tx = approve_result["data"]["txHash"]
        .as_str()
        .or_else(|| approve_result["txHash"].as_str())
        .or_else(|| approve_result["hash"].as_str())
        .unwrap_or("pending")
        .to_string();

    // Wait for approve tx to be mined before submitting supply
    if approve_tx != "pending" && approve_tx.starts_with("0x") {
        rpc::wait_for_tx(cfg.rpc_url, &approve_tx)
            .await
            .context("Approve tx did not confirm in time")?;
    }

    // Step 2: supply
    let supply_calldata = calldata::encode_supply(&token_addr, amount_minimal, &from_addr)
        .context("Failed to encode supply calldata")?;
    let supply_result = onchainos::wallet_contract_call(
        chain_id,
        &pool_addr,
        &supply_calldata,
        Some(&from_addr),
        false,
    )
    .context("Pool.supply() failed")?;
    let supply_tx = supply_result["data"]["txHash"]
        .as_str()
        .or_else(|| supply_result["txHash"].as_str())
        .or_else(|| supply_result["hash"].as_str())
        .unwrap_or("pending");

    Ok(json!({
        "ok": true,
        "asset": asset,
        "tokenAddress": token_addr,
        "amount": amount,
        "amountMinimal": amount_minimal.to_string(),
        "poolAddress": pool_addr,
        "approveTxHash": approve_tx,
        "supplyTxHash": supply_tx.to_string(),
        "dryRun": false
    }))
}

fn resolve_from(from: Option<&str>, chain_id: u64) -> anyhow::Result<String> {
    if let Some(addr) = from {
        return Ok(addr.to_string());
    }
    onchainos::wallet_address(chain_id).context("No --from address and could not resolve active wallet.")
}

#[allow(dead_code)]
/// Infer token decimals from well-known asset symbols.
/// Used when asset is a symbol (address-based resolution uses token search decimals).
pub fn infer_decimals(asset: &str) -> u64 {
    match asset.to_uppercase().as_str() {
        "USDC" | "USDT" | "USDC.E" | "USDBC" | "EURC" | "GHO" => 6,
        "WBTC" | "CBBTC" | "TBTC" => 8,
        "WETH" | "ETH" | "CBETH" | "WSTETH" | "RETH" | "WEETH" | "OSETH" => 18,
        _ => 18,
    }
}

pub fn human_to_minimal(amount: f64, decimals: u64) -> u128 {
    let factor = 10u128.pow(decimals as u32);
    (amount * factor as f64) as u128
}
