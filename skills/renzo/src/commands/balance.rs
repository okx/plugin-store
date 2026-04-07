use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct BalanceArgs {
    /// Address to query (defaults to current onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: BalanceArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    // Resolve address
    let address = match args.address {
        Some(a) => a,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Query ezETH balance
    let ezeth_calldata = rpc::calldata_balance_of(config::SEL_BALANCE_OF, &address);
    let ezeth_result = onchainos::eth_call(config::EZETH_ADDRESS, &ezeth_calldata, config::RPC_URL)?;
    let ezeth_raw = rpc::extract_return_data(&ezeth_result)?;
    let ezeth_wei = rpc::decode_uint256(&ezeth_raw).unwrap_or(0);

    // Query stETH balance
    let steth_calldata = rpc::calldata_balance_of(config::SEL_BALANCE_OF, &address);
    let steth_result = onchainos::eth_call(config::STETH_ADDRESS, &steth_calldata, config::RPC_URL)?;
    let steth_raw = rpc::extract_return_data(&steth_result)?;
    let steth_wei = rpc::decode_uint256(&steth_raw).unwrap_or(0);

    let ezeth_display = ezeth_wei as f64 / 1e18;
    let steth_display = steth_wei as f64 / 1e18;

    println!("{}", serde_json::json!({
        "ok": true,
        "data": {
            "address": address,
            "ezETH": {
                "balance": ezeth_display,
                "balance_wei": ezeth_wei.to_string(),
                "token": config::EZETH_ADDRESS
            },
            "stETH": {
                "balance": steth_display,
                "balance_wei": steth_wei.to_string(),
                "token": config::STETH_ADDRESS
            }
        }
    }));

    Ok(())
}
