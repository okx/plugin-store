use crate::{config, contracts::RocketPoolContracts, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct ApyArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,
}

pub async fn run(args: ApyArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    // Try Rocket Pool API first
    let api_apy = fetch_api_apy();

    // Always fetch on-chain rate for context
    let contracts = RocketPoolContracts::resolve(chain_id)?;
    let calldata = format!("0x{}", config::SEL_GET_EXCHANGE_RATE);
    let result = onchainos::eth_call(chain_id, &contracts.token_reth, &calldata)?;
    let data = rpc::extract_return_data(&result)?;
    let rate_wei = rpc::decode_uint256(&data)?;
    let rate_eth = rate_wei as f64 / 1e18;

    println!("=== Rocket Pool Staking APY ===");

    match api_apy {
        Ok(apy) => {
            println!("Current rETH APY:  {:.2}%", apy);
            println!("Source:            Rocket Pool API");
        }
        Err(_) => {
            // Fallback: display note that API is unavailable
            println!("APY:               N/A (API unavailable)");
            println!("Note: Check https://rocketpool.net for current APY");
        }
    }

    println!("Current rate:      1 rETH = {:.6} ETH", rate_eth);
    println!();
    println!("Notes:");
    println!("  - rETH is non-rebasing: your balance stays constant, rate increases");
    println!("  - APY reflects post-fee rate (14% node operator commission)");
    println!("  - No lock-up: rETH can be traded anytime on DEXes");

    Ok(())
}

fn fetch_api_apy() -> anyhow::Result<f64> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let resp: serde_json::Value = client
        .get(config::ROCKETPOOL_APR_API)
        .send()?
        .json()?;

    // Try different response shapes
    if let Some(apy) = resp["yearlyAPR"].as_f64() {
        return Ok(apy);
    }
    if let Some(apy) = resp["data"]["yearlyAPR"].as_f64() {
        return Ok(apy);
    }
    if let Some(apy) = resp["apr"].as_f64() {
        return Ok(apy);
    }
    if let Some(apy) = resp["data"]["apr"].as_f64() {
        return Ok(apy);
    }
    // Try parsing as a float string
    if let Some(s) = resp["yearlyAPR"].as_str() {
        if let Ok(v) = s.parse::<f64>() {
            return Ok(v);
        }
    }
    anyhow::bail!("Could not parse APY from API response: {}", resp)
}
