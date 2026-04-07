use crate::{config, onchainos, rpc};
use serde_json::json;

pub async fn run(chain_id: u64) -> anyhow::Result<()> {
    // ── swETH rates ──────────────────────────────────────────────────────────
    let sweth_to_eth_calldata = rpc::calldata_noarg(config::SEL_SWETH_TO_ETH_RATE);
    let eth_to_sweth_calldata = rpc::calldata_noarg(config::SEL_ETH_TO_SWETH_RATE);

    let sweth_to_eth_raw = onchainos::eth_call(chain_id, config::SWETH_ADDRESS, &sweth_to_eth_calldata)?;
    let eth_to_sweth_raw = onchainos::eth_call(chain_id, config::SWETH_ADDRESS, &eth_to_sweth_calldata)?;

    let sweth_to_eth_wei = rpc::decode_uint256(&rpc::extract_return_data(&sweth_to_eth_raw)?)?;
    let eth_to_sweth_wei = rpc::decode_uint256(&rpc::extract_return_data(&eth_to_sweth_raw)?)?;

    let sweth_to_eth = rpc::format_eth(sweth_to_eth_wei);
    let eth_to_sweth = rpc::format_eth(eth_to_sweth_wei);

    // ── rswETH rates ─────────────────────────────────────────────────────────
    let rsweth_to_eth_calldata = rpc::calldata_noarg(config::SEL_RSWETH_TO_ETH_RATE);
    let eth_to_rsweth_calldata = rpc::calldata_noarg(config::SEL_ETH_TO_RSWETH_RATE);

    let rsweth_to_eth_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &rsweth_to_eth_calldata)?;
    let eth_to_rsweth_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &eth_to_rsweth_calldata)?;

    let rsweth_to_eth_wei = rpc::decode_uint256(&rpc::extract_return_data(&rsweth_to_eth_raw)?)?;
    let eth_to_rsweth_wei = rpc::decode_uint256(&rpc::extract_return_data(&eth_to_rsweth_raw)?)?;

    let rsweth_to_eth = rpc::format_eth(rsweth_to_eth_wei);
    let eth_to_rsweth = rpc::format_eth(eth_to_rsweth_wei);

    let output = json!({
        "ok": true,
        "chain_id": chain_id,
        "swETH": {
            "contract": config::SWETH_ADDRESS,
            "swETH_per_ETH": eth_to_sweth,
            "ETH_per_swETH": sweth_to_eth,
            "description": "1 ETH = ~{swETH_per_ETH} swETH (liquid staking token)"
        },
        "rswETH": {
            "contract": config::RSWETH_ADDRESS,
            "rswETH_per_ETH": eth_to_rsweth,
            "ETH_per_rswETH": rsweth_to_eth,
            "description": "1 ETH = ~{rswETH_per_ETH} rswETH (liquid restaking via EigenLayer)"
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
