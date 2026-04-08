use serde_json::Value;

/// Fetch ether.fi protocol stats: APY, TVL, exchange rate.
/// Returns a JSON object with fields: apy, tvl, exchangeRate.
/// Falls back gracefully if the API is unavailable.
pub async fn fetch_stats() -> anyhow::Result<EtherFiStats> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()?;

    // Primary: portfolio v3 stats endpoint
    let url = "https://app.ether.fi/api/portfolio/v3";
    let result = client
        .get(url)
        .header("Accept", "application/json")
        .header("User-Agent", "etherfi-plugin/0.1.0")
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            let json: Value = resp.json().await.unwrap_or_default();
            let apy = extract_apy(&json);
            let tvl = extract_tvl(&json);
            let exchange_rate = extract_exchange_rate(&json);
            Ok(EtherFiStats { apy, tvl, exchange_rate })
        }
        _ => {
            // Fallback: try the rates endpoint
            let rates_url = "https://app.ether.fi/api/etherfi/rates";
            let rates_result = client
                .get(rates_url)
                .header("Accept", "application/json")
                .header("User-Agent", "etherfi-plugin/0.1.0")
                .send()
                .await;

            match rates_result {
                Ok(resp) if resp.status().is_success() => {
                    let json: Value = resp.json().await.unwrap_or_default();
                    let apy = extract_apy(&json);
                    let exchange_rate = extract_exchange_rate(&json);
                    Ok(EtherFiStats { apy, tvl: None, exchange_rate })
                }
                _ => {
                    // Return unknown stats rather than failing completely
                    Ok(EtherFiStats {
                        apy: None,
                        tvl: None,
                        exchange_rate: None,
                    })
                }
            }
        }
    }
}

/// Extract APY percentage from API JSON response.
fn extract_apy(json: &Value) -> Option<f64> {
    // Try various known field paths
    if let Some(v) = json["apyPct"].as_f64() { return Some(v); }
    if let Some(v) = json["apy"].as_f64() { return Some(v); }
    if let Some(v) = json["weEthApy"].as_f64() { return Some(v); }
    if let Some(v) = json["stakingApy"].as_f64() { return Some(v); }
    if let Some(s) = json["apyPct"].as_str() { return s.parse().ok(); }
    if let Some(s) = json["apy"].as_str() { return s.parse().ok(); }
    None
}

/// Extract TVL from API JSON response.
fn extract_tvl(json: &Value) -> Option<f64> {
    if let Some(v) = json["tvl"].as_f64() { return Some(v); }
    if let Some(s) = json["tvl"].as_str() { return s.parse().ok(); }
    None
}

/// Extract weETH/eETH exchange rate from API JSON response.
fn extract_exchange_rate(json: &Value) -> Option<f64> {
    if let Some(v) = json["exchangeRate"].as_f64() { return Some(v); }
    if let Some(v) = json["weEthToEEthRate"].as_f64() { return Some(v); }
    if let Some(s) = json["exchangeRate"].as_str() { return s.parse().ok(); }
    None
}

/// ether.fi protocol stats returned from the API.
#[derive(Debug)]
pub struct EtherFiStats {
    /// Annual Percentage Yield (e.g. 3.8 = 3.8%)
    pub apy: Option<f64>,
    /// Total Value Locked in USD
    pub tvl: Option<f64>,
    /// weETH per eETH exchange rate (how much eETH one weETH is worth)
    pub exchange_rate: Option<f64>,
}
