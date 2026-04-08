use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct BalanceArgs {
    /// Address to check balance for (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: BalanceArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    let address = args
        .address
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if address.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --address or ensure onchainos is logged in.");
    }

    // balanceOf(address)
    let balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &address);
    let balance_result =
        onchainos::eth_call(chain_id, config::STETH_ADDRESS, &balance_calldata)?;

    // sharesOf(address)
    let shares_calldata = rpc::calldata_single_address(config::SEL_SHARES_OF, &address);
    let shares_result =
        onchainos::eth_call(chain_id, config::STETH_ADDRESS, &shares_calldata)?;

    println!("=== Lido stETH Balance ===");
    println!("Address: {}", address);

    match rpc::extract_return_data(&balance_result) {
        Ok(hex) => match rpc::decode_uint256(&hex) {
            Ok(balance_wei) => {
                let balance_eth = balance_wei as f64 / 1e18;
                println!("stETH Balance: {:.6} stETH ({} wei)", balance_eth, balance_wei);
            }
            Err(e) => println!("stETH Balance: (decode error: {})", e),
        },
        Err(_) => println!("stETH Balance: {}", balance_result),
    }

    match rpc::extract_return_data(&shares_result) {
        Ok(hex) => match rpc::decode_uint256(&hex) {
            Ok(shares) => {
                println!("Shares:        {} (exact, no rounding)", shares);
            }
            Err(e) => println!("Shares:        (decode error: {})", e),
        },
        Err(_) => {}
    }

    println!();
    println!("Note: stETH is a rebasing token — balance grows daily without transfers.");

    Ok(())
}
