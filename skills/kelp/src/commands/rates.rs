use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct RatesArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,
}

pub async fn run(chain_id: u64) -> anyhow::Result<()> {
    println!("=== Kelp DAO rsETH Exchange Rates ===");
    println!("Chain: Ethereum ({})", chain_id);
    println!();

    // Fetch rsETH price from LRTOracle
    let price_calldata = rpc::calldata_no_params(config::SEL_RSETH_PRICE);
    let price_result = onchainos::eth_call(chain_id, config::ORACLE_ADDRESS, &price_calldata)?;

    match rpc::extract_return_data(&price_result) {
        Ok(hex) => match rpc::decode_uint256(&hex) {
            Ok(price_wei) => {
                let price_eth = price_wei as f64 / 1e18;
                println!("rsETH/ETH Price:   {:.8} ETH per rsETH", price_eth);
                println!("                   (from LRTOracle.rsETHPrice())");

                // Fetch USD price via CoinGecko for reference
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .user_agent("kelp-plugin/0.1.0")
                    .build()?;
                if let Ok(resp) = client.get(config::COINGECKO_API).send().await {
                    if resp.status().is_success() {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            if let Some(usd) = body["kelp-dao-restaked-eth"]["usd"].as_f64() {
                                println!("rsETH/USD Price:   ${:.2} USD", usd);
                                let eth_usd = usd / price_eth;
                                println!("Implied ETH/USD:   ${:.2} USD", eth_usd);
                            }
                        }
                    }
                }

                println!();
                // Also try to get the amount to mint for 1 ETH
                let one_eth: u128 = 1_000_000_000_000_000_000;
                let mint_calldata = rpc::calldata_get_rseth_amount(config::ETH_ASSET_ADDRESS, one_eth);
                if let Ok(mint_result) = onchainos::eth_call(chain_id, config::DEPOSIT_POOL_ADDRESS, &mint_calldata) {
                    if let Ok(mint_hex) = rpc::extract_return_data(&mint_result) {
                        if let Ok(mint_amount) = rpc::decode_uint256(&mint_hex) {
                            let mint_eth = mint_amount as f64 / 1e18;
                            println!("Deposit Rate:      1 ETH → {:.8} rsETH", mint_eth);
                            println!("                   (from LRTDepositPool.getRsETHAmountToMint)");
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error decoding rsETH price: {}", e);
                println!("Raw response: {}", price_result);
            }
        },
        Err(e) => {
            println!("Error fetching rsETH price from oracle: {}", e);
            println!("Falling back to CoinGecko...");

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("kelp-plugin/0.1.0")
                .build()?;
            if let Ok(resp) = client.get(config::COINGECKO_API).send().await {
                if resp.status().is_success() {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        let eth_price = body["kelp-dao-restaked-eth"]["eth"].as_f64().unwrap_or(0.0);
                        let usd_price = body["kelp-dao-restaked-eth"]["usd"].as_f64().unwrap_or(0.0);
                        println!("rsETH/ETH Price:   {:.8} ETH (CoinGecko)", eth_price);
                        println!("rsETH/USD Price:   ${:.2} USD (CoinGecko)", usd_price);
                    }
                }
            }
        }
    }

    println!();
    println!("Contracts:");
    println!("  LRTOracle:     {}", config::ORACLE_ADDRESS);
    println!("  LRTDepositPool: {}", config::DEPOSIT_POOL_ADDRESS);
    println!("  rsETH Token:   {}", config::RSETH_ADDRESS);

    Ok(())
}
