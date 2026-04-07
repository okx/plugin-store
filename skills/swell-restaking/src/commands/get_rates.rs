use crate::{config, onchainos, rpc};
use serde_json::json;

pub async fn run(chain_id: u64) -> anyhow::Result<()> {
    // ── rswETH rates ─────────────────────────────────────────────────────────
    let rsweth_to_eth_calldata = rpc::calldata_noarg(config::SEL_RSWETH_TO_ETH_RATE);
    let eth_to_rsweth_calldata = rpc::calldata_noarg(config::SEL_ETH_TO_RSWETH_RATE);
    let total_eth_calldata = rpc::calldata_noarg(config::SEL_TOTAL_ETH_DEPOSITED);
    let total_supply_calldata = rpc::calldata_noarg(config::SEL_TOTAL_SUPPLY);

    let rsweth_to_eth_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &rsweth_to_eth_calldata)?;
    let eth_to_rsweth_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &eth_to_rsweth_calldata)?;
    let total_eth_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &total_eth_calldata)?;
    let total_supply_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &total_supply_calldata)?;

    let rsweth_to_eth_wei = rpc::decode_uint256(&rpc::extract_return_data(&rsweth_to_eth_raw)?)?;
    let eth_to_rsweth_wei = rpc::decode_uint256(&rpc::extract_return_data(&eth_to_rsweth_raw)?)?;
    let total_eth_wei = rpc::decode_uint256(&rpc::extract_return_data(&total_eth_raw)?)?;
    let total_supply_wei = rpc::decode_uint256(&rpc::extract_return_data(&total_supply_raw)?)?;

    let output = json!({
        "ok": true,
        "chain_id": chain_id,
        "rswETH": {
            "contract": config::RSWETH_ADDRESS,
            "rswETH_per_ETH": rpc::format_eth(eth_to_rsweth_wei),
            "ETH_per_rswETH": rpc::format_eth(rsweth_to_eth_wei),
            "total_eth_deposited": rpc::format_eth(total_eth_wei),
            "total_supply": rpc::format_eth(total_supply_wei),
            "description": "1 ETH stakes as rswETH (EigenLayer restaking). Rate appreciates as rewards accrue."
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
