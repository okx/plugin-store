use reqwest::blocking::Client;
use serde_json::Value;
use crate::config;

pub fn get_client() -> anyhow::Result<Client> {
    Ok(Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

/// GET /chains — list all supported chains
pub fn get_chains() -> anyhow::Result<Value> {
    let client = get_client()?;
    let url = format!("{}/chains", config::API_BASE);
    let resp: Value = client.get(&url).send()?.json()?;
    Ok(resp)
}

/// POST /currencies/v1 — list supported tokens for a chain
pub fn get_currencies(chain_id: u64, limit: u32) -> anyhow::Result<Value> {
    let client = get_client()?;
    let url = format!("{}/currencies/v1", config::API_BASE);
    let body = serde_json::json!({
        "chainIds": [chain_id],
        "defaultList": true,
        "limit": limit
    });
    let resp: Value = client.post(&url).json(&body).send()?.json()?;
    Ok(resp)
}

/// POST /quote — get bridge/swap quote
pub fn get_quote(
    user: &str,
    origin_chain_id: u64,
    destination_chain_id: u64,
    origin_currency: &str,
    destination_currency: &str,
    amount: &str,
    recipient: Option<&str>,
) -> anyhow::Result<Value> {
    let client = get_client()?;
    let url = format!("{}/quote", config::API_BASE);
    let mut body = serde_json::json!({
        "user": user,
        "originChainId": origin_chain_id,
        "destinationChainId": destination_chain_id,
        "originCurrency": origin_currency,
        "destinationCurrency": destination_currency,
        "amount": amount,
        "tradeType": "EXACT_INPUT"
    });
    if let Some(r) = recipient {
        body["recipient"] = serde_json::json!(r);
    }
    let resp: Value = client.post(&url).json(&body).send()?.json()?;
    Ok(resp)
}

/// GET /intents/status — check bridge transaction status
pub fn get_status(request_id: &str) -> anyhow::Result<Value> {
    let client = get_client()?;
    let url = format!("{}/intents/status?requestId={}", config::API_BASE, request_id);
    let resp: Value = client.get(&url).send()?.json()?;
    Ok(resp)
}
