use anyhow::Context;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.relay.link";

// Native ETH address used by Relay
pub const NATIVE_ETH: &str = "0x0000000000000000000000000000000000000000";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayChain {
    pub id: u64,
    pub name: String,
    pub display_name: Option<String>,
    pub explorer_url: Option<String>,
    pub disabled: Option<bool>,
    pub currency: Option<ChainCurrency>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainCurrency {
    pub symbol: String,
    pub decimals: Option<u8>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub user: String,
    pub recipient: String,
    pub origin_chain_id: u64,
    pub destination_chain_id: u64,
    pub origin_currency: String,
    pub destination_currency: String,
    pub amount: String,
    pub trade_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub steps: Vec<Step>,
    pub fees: Option<serde_json::Value>,
    pub details: Option<Details>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: String,           // "approve" or "deposit"
    pub action: Option<String>,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub items: Vec<StepItem>,
    pub request_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StepItem {
    pub status: Option<String>,
    pub data: StepData,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StepData {
    pub from: Option<String>,
    pub to: String,
    pub data: String,
    pub value: Option<String>,
    pub chain_id: Option<u64>,
    pub gas: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub currency_in: Option<CurrencyDetail>,
    pub currency_out: Option<CurrencyDetail>,
    pub time_estimate: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyDetail {
    pub currency: Option<CurrencyInfo>,
    pub amount: Option<String>,
    pub amount_formatted: Option<String>,
    pub amount_usd: Option<String>,
    pub minimum_amount: Option<String>,
    pub minimum_amount_formatted: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyInfo {
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub address: Option<String>,
    pub chain_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    #[serde(rename = "inTxHashes")]
    pub in_tx_hashes: Option<Vec<String>>,
    #[serde(rename = "txHashes")]
    pub tx_hashes: Option<Vec<String>>,
    pub error: Option<String>,
}

pub async fn get_chains() -> anyhow::Result<Vec<RelayChain>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/chains", BASE_URL))
        .send().await
        .context("Failed to reach api.relay.link/chains")?;
    let body: serde_json::Value = resp.json().await?;
    // Response is either an array directly or {chains: [...]}
    let arr = if body.is_array() {
        body
    } else {
        body.get("chains").cloned().unwrap_or(serde_json::json!([]))
    };
    let chains: Vec<RelayChain> = serde_json::from_value(arr)
        .context("Failed to parse chains response")?;
    Ok(chains)
}

pub async fn get_quote(req: QuoteRequest) -> anyhow::Result<QuoteResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/quote", BASE_URL))
        .json(&req)
        .send().await
        .context("Failed to reach api.relay.link/quote")?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    if !status.is_success() {
        let msg = body.get("message")
            .or_else(|| body.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error from Relay API");
        anyhow::bail!("Relay API error ({}): {}", status, msg);
    }

    serde_json::from_value(body).context("Failed to parse quote response")
}

pub async fn get_status(request_id: &str) -> anyhow::Result<StatusResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/intents/status", BASE_URL))
        .query(&[("requestId", request_id)])
        .send().await
        .context("Failed to reach api.relay.link/intents/status")?;
    resp.json::<StatusResponse>().await
        .context("Failed to parse status response")
}

/// Resolve token symbol to address for a given chain.
/// Returns the address if recognised, otherwise treats the input as an address directly.
pub fn resolve_token(symbol_or_addr: &str, chain_id: u64) -> String {
    match symbol_or_addr.to_uppercase().as_str() {
        "ETH" | "WETH" => NATIVE_ETH.to_string(),
        "USDC" => match chain_id {
            1     => "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
            42161 => "0xaf88d065e77c8cc2239327c5edb3a432268e5831".to_string(),
            8453  => "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913".to_string(),
            10    => "0x0b2c639c533813f4aa9d7837caf62653d097ff85".to_string(),
            137   => "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359".to_string(),
            _     => symbol_or_addr.to_string(),
        },
        "USDT" => match chain_id {
            1     => "0xdac17f958d2ee523a2206206994597c13d831ec7".to_string(),
            42161 => "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9".to_string(),
            8453  => "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2".to_string(),
            10    => "0x94b008aa00579c1307b0ef2c499ad98a8ce58e58".to_string(),
            137   => "0xc2132d05d31c914a87c6611c10748aeb04b58e8f".to_string(),
            _     => symbol_or_addr.to_string(),
        },
        "DAI" => match chain_id {
            1     => "0x6b175474e89094c44da98b954eedeac495271d0f".to_string(),
            42161 => "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1".to_string(),
            8453  => "0x50c5725949a6f0c72e6c4a641f24049a917db0cb".to_string(),
            10    => "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1".to_string(),
            137   => "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063".to_string(),
            _     => symbol_or_addr.to_string(),
        },
        _ => symbol_or_addr.to_string(),
    }
}

/// Return a public RPC URL for a given chain ID.
fn rpc_url_for_chain(chain_id: u64) -> &'static str {
    match chain_id {
        1     => "https://eth.llamarpc.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        8453  => "https://mainnet.base.org",
        10    => "https://mainnet.optimism.io",
        137   => "https://polygon-rpc.com",
        _     => "https://eth.llamarpc.com",
    }
}

/// Return a block-explorer tx URL for a given chain ID and tx hash.
pub fn explorer_tx_url(chain_id: u64, tx_hash: &str) -> String {
    let base = match chain_id {
        1     => "https://etherscan.io/tx/",
        42161 => "https://arbiscan.io/tx/",
        8453  => "https://basescan.org/tx/",
        10    => "https://optimistic.etherscan.io/tx/",
        137   => "https://polygonscan.com/tx/",
        _     => "https://etherscan.io/tx/",
    };
    format!("{}{}", base, tx_hash)
}

/// Fetch native ETH balance for an address on a chain (returns raw wei as u128).
/// Returns 0 on any error so a failed balance fetch doesn't block the bridge preview.
pub async fn get_eth_balance(chain_id: u64, address: &str) -> u128 {
    let rpc = rpc_url_for_chain(chain_id);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [address, "latest"],
        "id": 1
    });
    let Ok(resp) = client.post(rpc).json(&body).send().await else { return 0; };
    let Ok(val) = resp.json::<serde_json::Value>().await else { return 0; };
    let hex = val["result"].as_str().unwrap_or("0x0");
    u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0)
}

pub fn token_symbol(addr: &str, _chain_id: u64) -> &'static str {
    match addr.to_lowercase().as_str() {
        "0x0000000000000000000000000000000000000000" => "ETH",
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => "USDC", // ETH
        "0xaf88d065e77c8cc2239327c5edb3a432268e5831" => "USDC", // ARB
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" => "USDC", // Base
        "0x0b2c639c533813f4aa9d7837caf62653d097ff85" => "USDC", // OP
        "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" => "USDC", // Polygon
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => "USDT", // ETH
        "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9" => "USDT", // ARB
        "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2" => "USDT", // Base
        "0x94b008aa00579c1307b0ef2c499ad98a8ce58e58" => "USDT", // OP
        "0xc2132d05d31c914a87c6611c10748aeb04b58e8f" => "USDT", // Polygon
        "0x6b175474e89094c44da98b954eedeac495271d0f" => "DAI",  // ETH
        "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1" => "DAI",  // ARB/OP
        "0x50c5725949a6f0c72e6c4a641f24049a917db0cb" => "DAI",  // Base
        "0x8f3cf7ad23cd3cadbd9735aff958023239c6a063" => "DAI",  // Polygon
        _ => "UNKNOWN",
    }
}
