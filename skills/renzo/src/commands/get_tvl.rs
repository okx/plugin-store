use crate::{config, onchainos, rpc};

pub async fn run() -> anyhow::Result<()> {
    // Call calculateTVLs() on RestakeManager
    // Returns (uint256[][] operatorDelegatorTvls, uint256[] tvls, uint256 totalTVL)
    // totalTVL is the last uint256 in the ABI-encoded return data
    let calldata = format!("0x{}", config::SEL_CALCULATE_TVLS);
    let result = onchainos::eth_call(config::RESTAKE_MANAGER, &calldata, config::RPC_URL)?;
    let raw = rpc::extract_return_data(&result)?;

    // The return data is complex ABI-encoded. The simplest extraction:
    // totalTVL is the last 32 bytes of the return data
    let hex = raw.trim_start_matches("0x");
    let total_tvl_wei: u128 = if hex.len() >= 64 {
        let last_word = &hex[hex.len() - 64..];
        u128::from_str_radix(last_word, 16).unwrap_or(0)
    } else {
        0
    };

    let total_tvl_eth = total_tvl_wei as f64 / 1e18;

    // Also get ezETH total supply
    let supply_calldata = format!("0x{}", config::SEL_TOTAL_SUPPLY);
    let supply_result = onchainos::eth_call(config::EZETH_ADDRESS, &supply_calldata, config::RPC_URL)?;
    let supply_raw = rpc::extract_return_data(&supply_result)?;
    let total_supply_wei = rpc::decode_uint256(&supply_raw).unwrap_or(0);
    let total_supply_eth = total_supply_wei as f64 / 1e18;

    println!("{}", serde_json::json!({
        "ok": true,
        "data": {
            "total_tvl_eth": total_tvl_eth,
            "total_tvl_wei": total_tvl_wei.to_string(),
            "ezeth_total_supply": total_supply_eth,
            "ezeth_total_supply_wei": total_supply_wei.to_string(),
            "chain": config::CHAIN_ID,
            "restake_manager": config::RESTAKE_MANAGER
        }
    }));

    Ok(())
}
