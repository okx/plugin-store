use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct PositionsArgs {
    /// Address to check positions for (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub address: Option<String>,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    let address = if let Some(a) = args.address {
        a
    } else {
        let resolved = onchainos::resolve_wallet(chain_id, false)?;
        if resolved.is_empty() {
            anyhow::bail!("Cannot get wallet address. Pass --address or ensure onchainos is logged in.");
        }
        resolved
    };

    println!("=== Kelp DAO rsETH Positions ===");
    println!("Address: {}", address);
    println!("Chain:   Ethereum ({})", chain_id);
    println!();

    // 1. Fetch rsETH balance
    let balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &address);
    let balance_result = onchainos::eth_call(chain_id, config::RSETH_ADDRESS, &balance_calldata)?;

    let rseth_balance_wei = match rpc::extract_return_data(&balance_result) {
        Ok(hex) => rpc::decode_uint256(&hex).unwrap_or(0),
        Err(_) => 0,
    };
    let rseth_balance = rseth_balance_wei as f64 / 1e18;

    println!("rsETH Balance:     {:.8} rsETH ({} wei)", rseth_balance, rseth_balance_wei);

    // 2. Fetch rsETH/ETH price from oracle
    let price_calldata = rpc::calldata_no_params(config::SEL_RSETH_PRICE);
    let price_result = onchainos::eth_call(chain_id, config::ORACLE_ADDRESS, &price_calldata)?;

    let price_eth = match rpc::extract_return_data(&price_result) {
        Ok(hex) => match rpc::decode_uint256(&hex) {
            Ok(p) => p as f64 / 1e18,
            Err(_) => 1.0,
        },
        Err(_) => 1.0,
    };

    println!("rsETH/ETH Rate:    {:.8} ETH per rsETH (LRTOracle)", price_eth);

    // 3. Compute ETH value
    let eth_value = rseth_balance * price_eth;
    println!("ETH Value:         {:.8} ETH", eth_value);

    // 4. Fetch USD price for display
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("kelp-plugin/0.1.0")
        .build()?;
    if let Ok(resp) = client.get(config::COINGECKO_API).send().await {
        if resp.status().is_success() {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                if let Some(usd_per_rseth) = body["kelp-dao-restaked-eth"]["usd"].as_f64() {
                    let usd_value = rseth_balance * usd_per_rseth;
                    println!("USD Value:         ${:.2}", usd_value);
                }
            }
        }
    }

    println!();
    if rseth_balance_wei == 0 {
        println!("No rsETH holdings found for this address.");
        println!("Deposit ETH or LSTs using: kelp stake --amount <ETH> --chain {}", chain_id);
    } else {
        println!("To unstake rsETH: kelp unstake --amount <AMOUNT> --chain {}", chain_id);
        println!("Note: rsETH value increases over time as restaking rewards accrue.");
    }

    Ok(())
}
