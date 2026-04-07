use crate::{config, contracts::RocketPoolContracts, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StatsArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,
}

pub async fn run(args: StatsArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;
    let contracts = RocketPoolContracts::resolve(chain_id)?;

    // 1. Total ETH staked (TVL)
    let total_eth_wei = {
        let calldata = format!("0x{}", config::SEL_GET_TOTAL_ETH);
        let result = onchainos::eth_call(chain_id, &contracts.network_balances, &calldata)?;
        let data = rpc::extract_return_data(&result)?;
        rpc::decode_uint256(&data).unwrap_or(0)
    };

    // 2. Total rETH supply
    let total_reth_wei = {
        let calldata = format!("0x{}", config::SEL_TOTAL_SUPPLY);
        let result = onchainos::eth_call(chain_id, &contracts.token_reth, &calldata)?;
        let data = rpc::extract_return_data(&result)?;
        rpc::decode_uint256(&data).unwrap_or(0)
    };

    // 3. Exchange rate
    let rate_wei = {
        let calldata = format!("0x{}", config::SEL_GET_EXCHANGE_RATE);
        let result = onchainos::eth_call(chain_id, &contracts.token_reth, &calldata)?;
        let data = rpc::extract_return_data(&result)?;
        rpc::decode_uint256(&data).unwrap_or(0)
    };

    // 4. Node count
    let node_count = {
        let calldata = format!("0x{}", config::SEL_GET_NODE_COUNT);
        let result = onchainos::eth_call(chain_id, &contracts.node_manager, &calldata)?;
        let data = rpc::extract_return_data(&result)?;
        rpc::decode_uint256(&data).unwrap_or(0)
    };

    // 5. Minipool count
    let minipool_count = {
        let calldata = format!("0x{}", config::SEL_GET_MINIPOOL_COUNT);
        let result = onchainos::eth_call(chain_id, &contracts.minipool_manager, &calldata)?;
        let data = rpc::extract_return_data(&result)?;
        rpc::decode_uint256(&data).unwrap_or(0)
    };

    // 6. Deposit pool balance
    let deposit_pool_eth_wei = {
        let calldata = format!("0x{}", config::SEL_GET_DEPOSIT_BALANCE);
        let result = onchainos::eth_call(chain_id, &contracts.deposit_pool, &calldata)?;
        let data = rpc::extract_return_data(&result)?;
        rpc::decode_uint256(&data).unwrap_or(0)
    };

    let total_eth = total_eth_wei as f64 / 1e18;
    let total_reth = total_reth_wei as f64 / 1e18;
    let rate = rate_wei as f64 / 1e18;
    let deposit_pool_eth = deposit_pool_eth_wei as f64 / 1e18;

    println!("=== Rocket Pool Protocol Stats ===");
    println!("Chain: Ethereum Mainnet (ID: {})", chain_id);
    println!();
    println!("TVL (Total ETH Staked): {:.2} ETH", total_eth);
    println!("Total rETH Supply:      {:.4} rETH", total_reth);
    println!("Exchange Rate:          1 rETH = {:.6} ETH", rate);
    println!("Deposit Pool Balance:   {:.4} ETH", deposit_pool_eth);
    println!("Node Operators:         {}", node_count);
    println!("Active Minipools:       {}", minipool_count);
    println!();
    println!("Contracts (resolved dynamically via RocketStorage):");
    println!("  RocketStorage:        {}", config::ROCKET_STORAGE);
    println!("  RocketDepositPool:    {}", contracts.deposit_pool);
    println!("  RocketTokenRETH:      {}", contracts.token_reth);
    println!("  RocketNetworkBal:     {}", contracts.network_balances);

    Ok(())
}
