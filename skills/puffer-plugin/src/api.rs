use serde_json::Value;

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

/// Fetch Puffer Finance pufETH APY from DeFiLlama yields API.
/// Pool id: `90bfb3c2-5d35-4959-a275-83a22f1d85f1` (puffer-finance → pufETH, Ethereum).
/// Falls back to None if API is unavailable.
pub async fn fetch_pufeth_apy() -> Option<f64> {
    // DeFiLlama pool id for puffer-stake pufETH on Ethereum.
    const POOL_ID: &str = "bac6982a-f344-42f7-9af4-a9882f4a77f0";
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;

    let url = format!("https://yields.llama.fi/chart/{}", POOL_ID);
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: Value = resp.json().await.ok()?;
    json["data"]
        .as_array()
        .and_then(|a| a.last())
        .and_then(|v| v["apy"].as_f64())
}
