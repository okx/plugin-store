use serde_json::Value;

/// DeFiLlama pool ID for ether.fi weETH staking (Ethereum mainnet).
/// Source: https://yields.llama.fi/pools — project "ether.fi-stake", symbol "WEETH"
const DEFILLAMA_POOL_ID: &str = "46bd2bdf-6d92-4066-b482-e885ee172264";

/// Fetch current ETH/USD price from DeFiLlama coins API.
/// Returns None if the API is unavailable — callers should degrade gracefully.
pub async fn fetch_eth_price() -> Option<f64> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;

    let resp = client
        .get("https://coins.llama.fi/prices/current/coingecko:ethereum")
        .header("Accept", "application/json")
        .send()
        .await
        .ok()?;

    let json: Value = resp.json().await.ok()?;
    json["coins"]["coingecko:ethereum"]["price"].as_f64()
}

/// Fetch ether.fi protocol stats: APY and TVL via DeFiLlama.
/// Exchange rate is read on-chain via weETH.getRate() in rpc.rs.
/// Falls back gracefully if the API is unavailable.
pub async fn fetch_stats() -> anyhow::Result<EtherFiStats> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()?;

    let url = format!("https://yields.llama.fi/chart/{}", DEFILLAMA_POOL_ID);
    let result = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            let json: Value = resp.json().await.unwrap_or_default();
            // /chart returns {"status":"ok","data":[...]} — take last entry
            if let Some(latest) = json["data"].as_array().and_then(|a| a.last()) {
                let apy = latest["apy"].as_f64();
                let tvl = latest["tvlUsd"].as_f64();
                return Ok(EtherFiStats { apy, tvl });
            }
            Ok(EtherFiStats { apy: None, tvl: None })
        }
        _ => Ok(EtherFiStats { apy: None, tvl: None }),
    }
}

/// ether.fi protocol stats returned from the API.
#[derive(Debug)]
pub struct EtherFiStats {
    /// Annual Percentage Yield (e.g. 2.77 = 2.77%)
    pub apy: Option<f64>,
    /// Total Value Locked in USD
    pub tvl: Option<f64>,
}
