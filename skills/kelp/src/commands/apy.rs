use crate::config;

pub async fn run() -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("kelp-plugin/0.1.0")
        .build()?;

    // Try CoinGecko for price and 24h change data
    let resp = client
        .get(config::COINGECKO_API)
        .send()
        .await?;

    println!("=== Kelp DAO rsETH APY ===");

    if resp.status().is_success() {
        let body: serde_json::Value = resp.json().await?;
        let eth_price = body["kelp-dao-restaked-eth"]["eth"].as_f64();
        let usd_price = body["kelp-dao-restaked-eth"]["usd"].as_f64();
        let eth_24h_change = body["kelp-dao-restaked-eth"]["eth_24h_change"].as_f64();

        if let (Some(eth), Some(usd)) = (eth_price, usd_price) {
            println!("rsETH Price:       {:.6} ETH  (${:.2} USD)", eth, usd);
        }

        // Try to get more detailed APY from CoinGecko coins endpoint
        let coins_resp = client
            .get(config::KELP_APY_API)
            .send()
            .await;

        if let Ok(cr) = coins_resp {
            if cr.status().is_success() {
                let coins_body: serde_json::Value = cr.json().await?;

                // Extract staking yield from market_data if available
                if let Some(yield_val) = coins_body["market_data"]["current_price"]["eth"].as_f64() {
                    // APY approximation: since rsETH accrues value vs ETH,
                    // the ratio increase represents yield.
                    // rsETH/ETH ratio > 1.0 means restaking rewards have been earned.
                    println!("rsETH/ETH Ratio:   {:.6}", yield_val);
                }

                if let Some(change_7d) = coins_body["market_data"]["price_change_percentage_7d_in_currency"]["eth"].as_f64() {
                    // Annualize 7-day change as APY estimate
                    let apy_estimate = change_7d * (365.0 / 7.0);
                    println!("Estimated APY:     {:.2}% (annualized from 7d ETH price change)", apy_estimate);
                } else if let Some(change_24h) = eth_24h_change {
                    let apy_estimate = change_24h * 365.0;
                    println!("Estimated APY:     ~{:.2}% (annualized from 24h change — indicative only)", apy_estimate);
                } else {
                    println!("Estimated APY:     ~4-5% (restaking + staking rewards, check kelpdao.xyz for latest)");
                }
            }
        }
    } else {
        println!("Estimated APY:     ~4-5% (restaking + staking rewards)");
        println!("Live APY:          Check https://kerneldao.com/kelp/ for current rates");
    }

    println!();
    println!("Yield Sources:");
    println!("  • EigenLayer restaking rewards");
    println!("  • Underlying LST staking rewards (stETH, ETHx, sfrxETH)");
    println!("  • Kelp DAO points (KELP token allocation)");
    println!("Note: APY is variable and depends on EigenLayer operator performance.");

    Ok(())
}
