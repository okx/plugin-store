// commands/quote.rs — Get a Curve swap quote via pool get_dy() (direct pool call)
use crate::{api, config, curve_abi, rpc};
use anyhow::Result;

/// Determine whether a pool uses uint256 or int128 indices.
/// Factory v2 (CryptoSwap, tricrypto) pools use uint256; classic StableSwap pools use int128.
/// NOTE: Some old-style CryptoSwap pools are registered in the main registry with numeric IDs
/// (e.g. id="38" for the USDT/WBTC/WETH pool). We therefore try uint256 first and fall back
/// to int128 when the uint256 call returns empty data.
fn uses_uint256_indices(pool: &api::PoolData) -> bool {
    let id = pool.id.to_lowercase();
    id.contains("factory-crypto") || id.contains("tricrypto") || id.contains("crypto")
}

pub async fn run(
    chain_id: u64,
    token_in: String,
    token_out: String,
    amount_in: u128,
    slippage: f64,
) -> Result<()> {
    let chain_name = config::chain_name(chain_id);
    let rpc_url = config::rpc_url(chain_id);

    let token_in_addr = config::resolve_token_address(&token_in, chain_id);
    let token_out_addr = config::resolve_token_address(&token_out, chain_id);

    // Fetch pools and find one containing both tokens
    let pools = api::get_all_pools(chain_name).await?;
    let matching_pools = api::find_pools_for_pair(&pools, &token_in_addr, &token_out_addr);

    if matching_pools.is_empty() {
        anyhow::bail!(
            "No Curve pool found on {} containing both {} and {}",
            chain_name,
            token_in,
            token_out
        );
    }

    // Use first matching pool (highest TVL since list is sorted desc)
    let pool = matching_pools[0];
    let in_idx = api::coin_index(pool, &token_in_addr).unwrap_or(0);
    let out_idx = api::coin_index(pool, &token_out_addr).unwrap_or(1);

    // Try uint256 selector first (covers crypto/factory-crypto pools and old-style CryptoSwap
    // pools that appear in the main registry with numeric IDs). Fall back to int128 for classic
    // StableSwap pools.
    let (calldata, actual_uint256) = if uses_uint256_indices(pool) {
        (curve_abi::encode_get_dy_uint256(in_idx as u64, out_idx as u64, amount_in), true)
    } else {
        // Try uint256 first; if it returns empty fall back to int128
        let cd_u256 = curve_abi::encode_get_dy_uint256(in_idx as u64, out_idx as u64, amount_in);
        let hex_u256 = rpc::eth_call(&pool.address, &cd_u256, rpc_url).await.unwrap_or_default();
        let val_u256 = rpc::decode_uint128(&hex_u256);
        if val_u256 > 0 {
            (cd_u256, true)
        } else {
            (curve_abi::encode_get_dy(in_idx as i64, out_idx as i64, amount_in), false)
        }
    };
    let _ = actual_uint256; // may be used in future for exchange selector selection

    let result_hex = rpc::eth_call(&pool.address, &calldata, rpc_url).await?;
    let amount_out = rpc::decode_uint128(&result_hex);

    if amount_out == 0 {
        anyhow::bail!("Quote returned 0 — pool may have insufficient liquidity for this pair");
    }

    // Calculate min_expected with slippage
    let min_expected = (amount_out as f64 * (1.0 - slippage)) as u128;
    let price_impact_pct = if amount_out > 0 {
        let in_f = amount_in as f64;
        let out_f = amount_out as f64;
        ((in_f - out_f) / in_f * 100.0).max(0.0)
    } else {
        0.0
    };

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": chain_name,
            "pool": { "id": pool.id, "name": pool.name, "address": pool.address },
            "token_in": { "symbol": token_in, "address": token_in_addr, "index": in_idx },
            "token_out": { "symbol": token_out, "address": token_out_addr, "index": out_idx },
            "amount_in_raw": amount_in.to_string(),
            "amount_out_raw": amount_out.to_string(),
            "min_expected_raw": min_expected.to_string(),
            "slippage_pct": slippage * 100.0,
            "price_impact_pct": format!("{:.4}", price_impact_pct),
        })
    );
    Ok(())
}
