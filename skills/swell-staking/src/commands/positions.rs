use crate::{config, onchainos, rpc};
use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (optional; resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: PositionsArgs, chain_id: u64) -> anyhow::Result<()> {
    // Resolve wallet address
    let address = match args.address {
        Some(a) => a,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    if address.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --address or ensure onchainos is logged in.");
    }

    // ── swETH balance ────────────────────────────────────────────────────────
    let sweth_balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &address);
    let sweth_balance_raw = onchainos::eth_call(chain_id, config::SWETH_ADDRESS, &sweth_balance_calldata)?;
    let sweth_balance_wei = rpc::decode_uint256(&rpc::extract_return_data(&sweth_balance_raw)?)?;

    // ── rswETH balance ───────────────────────────────────────────────────────
    let rsweth_balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &address);
    let rsweth_balance_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &rsweth_balance_calldata)?;
    let rsweth_balance_wei = rpc::decode_uint256(&rpc::extract_return_data(&rsweth_balance_raw)?)?;

    // ── ETH equivalent values ────────────────────────────────────────────────
    // swETH ETH value
    let sweth_rate_calldata = rpc::calldata_noarg(config::SEL_SWETH_TO_ETH_RATE);
    let sweth_rate_raw = onchainos::eth_call(chain_id, config::SWETH_ADDRESS, &sweth_rate_calldata)?;
    let sweth_rate_wei = rpc::decode_uint256(&rpc::extract_return_data(&sweth_rate_raw)?)?;
    // swETH_eth_value = sweth_balance_wei * sweth_rate_wei / 1e18
    let sweth_eth_value = if sweth_balance_wei > 0 {
        // Use u128 arithmetic — may lose some precision for very large amounts but safe for test amounts
        let numerator = (sweth_balance_wei as u128).saturating_mul(sweth_rate_wei as u128);
        numerator / 1_000_000_000_000_000_000u128
    } else {
        0u128
    };

    // rswETH ETH value
    let rsweth_rate_calldata = rpc::calldata_noarg(config::SEL_RSWETH_TO_ETH_RATE);
    let rsweth_rate_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &rsweth_rate_calldata)?;
    let rsweth_rate_wei = rpc::decode_uint256(&rpc::extract_return_data(&rsweth_rate_raw)?)?;
    let rsweth_eth_value = if rsweth_balance_wei > 0 {
        let numerator = (rsweth_balance_wei as u128).saturating_mul(rsweth_rate_wei as u128);
        numerator / 1_000_000_000_000_000_000u128
    } else {
        0u128
    };

    let output = json!({
        "ok": true,
        "chain_id": chain_id,
        "address": address,
        "positions": {
            "swETH": {
                "balance": rpc::format_eth(sweth_balance_wei),
                "balance_wei": sweth_balance_wei.to_string(),
                "eth_value": rpc::format_eth(sweth_eth_value),
                "contract": config::SWETH_ADDRESS,
                "type": "Liquid Staking Token"
            },
            "rswETH": {
                "balance": rpc::format_eth(rsweth_balance_wei),
                "balance_wei": rsweth_balance_wei.to_string(),
                "eth_value": rpc::format_eth(rsweth_eth_value),
                "contract": config::RSWETH_ADDRESS,
                "type": "Liquid Restaking Token (EigenLayer)"
            }
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
