use anyhow::Context;
use serde_json::{json, Value};

use crate::config;
use crate::rpc;

/// List SparkLend reserve data — supply APYs, variable borrow APYs, asset addresses.
///
/// Calls Pool.getReservesList() to obtain asset addresses, then queries each asset
/// via Pool.getReserveData(address) (selector 0x35ea6a75) which returns the packed
/// DataTypes.ReserveData struct:
///
///   Slot 0: configuration (uint256, packed bitmask)
///   Slot 1: liquidityIndex (ray = 1e27)
///   Slot 2: currentLiquidityRate  ← supply APY (ray = 1e27)
///   Slot 3: variableBorrowIndex (ray)
///   Slot 4: currentVariableBorrowRate  ← variable borrow APY (ray = 1e27)
pub async fn run(
    chain_id: u64,
    asset_filter: Option<&str>,
) -> anyhow::Result<Value> {
    // Resolve Pool address at runtime
    let pool_addr = rpc::get_pool(config::POOL_ADDRESSES_PROVIDER, config::RPC_URL)
        .await
        .context("Failed to resolve SparkLend Pool address")?;

    // Get list of reserves from Pool.getReservesList()
    let reserves_list_hex = rpc::eth_call(config::RPC_URL, &pool_addr, "0xd1946dbc")
        .await
        .context("Failed to call Pool.getReservesList()")?;

    let reserve_addresses = decode_address_array(&reserves_list_hex)?;

    if reserve_addresses.is_empty() {
        return Ok(json!({
            "ok": true,
            "chain": config::CHAIN_NAME,
            "chainId": chain_id,
            "reserves": [],
            "message": "No reserves found"
        }));
    }

    let mut reserves: Vec<Value> = Vec::new();

    for addr in &reserve_addresses {
        let symbol = rpc::get_erc20_symbol(addr, config::RPC_URL).await.unwrap_or_default();

        // Apply filter: match by address (0x...) or symbol (case-insensitive)
        if let Some(filter) = asset_filter {
            if filter.starts_with("0x") {
                if !addr.eq_ignore_ascii_case(filter) {
                    continue;
                }
            } else if !symbol.eq_ignore_ascii_case(filter) {
                continue;
            }
        }

        match get_reserve_data_from_pool(&pool_addr, addr, &symbol, config::RPC_URL).await {
            Ok(reserve_data) => {
                reserves.push(reserve_data);
            }
            Err(e) => {
                eprintln!("Warning: failed to fetch data for reserve {}: {}", addr, e);
            }
        }
    }

    Ok(json!({
        "ok": true,
        "chain": config::CHAIN_NAME,
        "chainId": chain_id,
        "reserveCount": reserves.len(),
        "reserves": reserves
    }))
}

/// Fetch reserve data from Pool.getReserveData(address) — selector 0x35ea6a75.
async fn get_reserve_data_from_pool(
    pool_addr: &str,
    asset_addr: &str,
    symbol: &str,
    rpc_url: &str,
) -> anyhow::Result<Value> {
    let addr_bytes = hex::decode(asset_addr.trim_start_matches("0x"))?;
    let mut data = hex::decode("35ea6a75")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);
    let data_hex = format!("0x{}", hex::encode(&data));

    let result = rpc::eth_call(rpc_url, pool_addr, &data_hex).await?;
    let raw = result.trim_start_matches("0x");

    if raw.len() < 64 * 5 {
        anyhow::bail!("Pool.getReserveData: short response ({} chars)", raw.len());
    }

    // Slot 2: currentLiquidityRate (supply APY, ray = 1e27)
    let liquidity_rate = decode_ray_to_apy_pct(raw, 2)?;
    // Slot 4: currentVariableBorrowRate (variable borrow APY, ray = 1e27)
    let variable_borrow_rate = decode_ray_to_apy_pct(raw, 4)?;

    Ok(json!({
        "symbol": symbol,
        "underlyingAsset": asset_addr,
        "supplyApy": format!("{:.4}%", liquidity_rate),
        "variableBorrowApy": format!("{:.4}%", variable_borrow_rate)
    }))
}

fn decode_ray_to_apy_pct(raw: &str, slot: usize) -> anyhow::Result<f64> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        return Ok(0.0);
    }
    let slot_hex = &raw[start..end];
    let low = &slot_hex[32..64];
    let val = u128::from_str_radix(low, 16).unwrap_or(0);
    let pct = val as f64 / 1e27 * 100.0;
    Ok(pct)
}

/// Decode an ABI-encoded dynamic array of addresses.
/// ABI encoding: offset (32), length (32), then N x address (32 each)
fn decode_address_array(hex_result: &str) -> anyhow::Result<Vec<String>> {
    let raw = hex_result.trim_start_matches("0x");
    if raw.len() < 128 {
        return Ok(vec![]);
    }
    let len_hex = &raw[64..128];
    let len = usize::from_str_radix(len_hex.trim_start_matches('0'), 16).unwrap_or(0);
    if len == 0 {
        return Ok(vec![]);
    }

    let mut addresses = Vec::with_capacity(len);
    let data_start = 128;
    for i in 0..len {
        let slot_start = data_start + i * 64;
        let slot_end = slot_start + 64;
        if raw.len() < slot_end {
            break;
        }
        let addr_hex = &raw[slot_end - 40..slot_end];
        addresses.push(format!("0x{}", addr_hex));
    }
    Ok(addresses)
}
