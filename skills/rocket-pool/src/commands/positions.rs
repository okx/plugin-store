use crate::{config, contracts::RocketPoolContracts, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct PositionsArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Address to query (resolved from onchainos wallet if omitted)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;
    let contracts = RocketPoolContracts::resolve(chain_id)?;

    // Resolve address
    let address = match args.address {
        Some(a) => a,
        None => onchainos::resolve_wallet(chain_id, false)
            .map_err(|e| anyhow::anyhow!("Cannot resolve wallet address: {}. Pass --address or ensure onchainos is logged in.", e))?,
    };

    // rETH balance
    let calldata = format!(
        "0x{}{}",
        config::SEL_BALANCE_OF,
        rpc::encode_address(&address)
    );
    let result = onchainos::eth_call(chain_id, &contracts.token_reth, &calldata)?;
    let data = rpc::extract_return_data(&result)?;
    let reth_balance_wei = rpc::decode_uint256(&data).unwrap_or(0);

    // Exchange rate
    let rate_calldata = format!("0x{}", config::SEL_GET_EXCHANGE_RATE);
    let rate_result = onchainos::eth_call(chain_id, &contracts.token_reth, &rate_calldata)?;
    let rate_data = rpc::extract_return_data(&rate_result)?;
    let rate_wei = rpc::decode_uint256(&rate_data).unwrap_or(0);

    let reth_balance = reth_balance_wei as f64 / 1e18;
    let rate = rate_wei as f64 / 1e18;
    let eth_equivalent = reth_balance * rate;

    println!("=== Rocket Pool Position ===");
    println!("Address:       {}", address);
    println!("Chain:         Ethereum Mainnet (ID: {})", chain_id);
    println!();

    if reth_balance_wei == 0 {
        println!("rETH Balance:  0 rETH");
        println!("ETH Value:     0 ETH");
        println!();
        println!("No rETH position found. To stake ETH for rETH, use:");
        println!("  rocket-pool stake --amount 0.01");
    } else {
        println!("rETH Balance:  {:.6} rETH", reth_balance);
        println!("ETH Equivalent: {:.6} ETH (at {:.6} ETH/rETH)", eth_equivalent, rate);
        println!("rETH in wei:   {}", reth_balance_wei);
        println!();
        println!("rETH contract: {}", contracts.token_reth);
        println!();
        println!("To unstake, use:");
        println!("  rocket-pool unstake --amount {:.6}", reth_balance);
    }

    Ok(())
}
