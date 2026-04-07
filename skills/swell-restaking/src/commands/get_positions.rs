use crate::{config, onchainos, rpc};
use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct GetPositionsArgs {
    /// Wallet address to query (optional; resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: GetPositionsArgs, chain_id: u64) -> anyhow::Result<()> {
    // Resolve wallet address
    let address = match args.address {
        Some(a) => a,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    if address.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --address or ensure onchainos is logged in.");
    }

    // ── rswETH balance ───────────────────────────────────────────────────────
    let balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &address);
    let balance_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &balance_calldata)?;
    let balance_wei = rpc::decode_uint256(&rpc::extract_return_data(&balance_raw)?)?;

    // ── ETH equivalent value via rswETHToETHRate ─────────────────────────────
    let rate_calldata = rpc::calldata_noarg(config::SEL_RSWETH_TO_ETH_RATE);
    let rate_raw = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &rate_calldata)?;
    let rate_wei = rpc::decode_uint256(&rpc::extract_return_data(&rate_raw)?)?;

    let eth_value = if balance_wei > 0 {
        let numerator = (balance_wei as u128).saturating_mul(rate_wei as u128);
        numerator / 1_000_000_000_000_000_000u128
    } else {
        0u128
    };

    let output = json!({
        "ok": true,
        "chain_id": chain_id,
        "address": address,
        "positions": {
            "rswETH": {
                "balance": rpc::format_eth(balance_wei),
                "balance_wei": balance_wei.to_string(),
                "eth_value": rpc::format_eth(eth_value),
                "eth_value_wei": eth_value.to_string(),
                "contract": config::RSWETH_ADDRESS,
                "type": "Liquid Restaking Token (EigenLayer)"
            }
        },
        "note": "To unstake rswETH, use the Swell app at https://app.swellnetwork.io (1-12 day withdrawal period)"
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
