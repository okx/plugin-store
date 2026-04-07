use crate::{config, contracts::RocketPoolContracts, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct RateArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,
}

pub async fn run(args: RateArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;
    let contracts = RocketPoolContracts::resolve(chain_id)?;

    let calldata = format!("0x{}", config::SEL_GET_EXCHANGE_RATE);
    let result = onchainos::eth_call(chain_id, &contracts.token_reth, &calldata)?;
    let data = rpc::extract_return_data(&result)?;
    let rate_wei = rpc::decode_uint256(&data)?;

    let rate_eth = rate_wei as f64 / 1e18;

    println!("=== Rocket Pool rETH Exchange Rate ===");
    println!("rETH contract: {}", contracts.token_reth);
    println!("1 rETH = {:.6} ETH", rate_eth);
    println!("Rate (wei):    {}", rate_wei);
    println!();
    println!("Note: rETH appreciates over time as staking rewards accumulate.");
    println!("      Your rETH balance stays constant, but each rETH is worth more ETH.");

    Ok(())
}
