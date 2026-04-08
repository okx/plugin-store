/// pools command — list top Balancer V2 pools via Balancer API

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Balancer API v3 GraphQL endpoint (accessible from sandbox)
const BALANCER_API: &str = "https://api-v3.balancer.fi/graphql";

fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        42161 => "ARBITRUM",
        1 => "MAINNET",
        137 => "POLYGON",
        8453 => "BASE",
        _ => "ARBITRUM",
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolToken {
    pub address: String,
    pub symbol: String,
    pub decimals: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PoolInfo {
    pub id: String,
    pub address: String,
    pub pool_type: String,
    pub total_liquidity_usd: String,
    pub swap_fee: String,
    pub tokens: Vec<PoolToken>,
}

pub async fn run(chain_id: u64, limit: usize) -> Result<()> {
    let chain = chain_name(chain_id);

    let query = format!(
        r#"{{"query": "{{ poolGetPools(first: {limit}, where: {{ chainIn: [{chain}] }}, orderBy: totalLiquidity, orderDirection: desc) {{ id address chain type dynamicData {{ totalLiquidity swapFee }} poolTokens {{ address symbol decimals weight }} }} }}"}}"#,
        limit = limit,
        chain = chain,
    );

    let client = reqwest::Client::new();
    let resp: Value = client
        .post(BALANCER_API)
        .header("Content-Type", "application/json")
        .body(query)
        .send()
        .await?
        .json()
        .await?;

    if let Some(errors) = resp.get("errors") {
        anyhow::bail!("Balancer API errors: {}", errors);
    }

    let pools_raw = &resp["data"]["poolGetPools"];
    if !pools_raw.is_array() {
        anyhow::bail!("No pools data returned from Balancer API");
    }

    let mut pools: Vec<PoolInfo> = Vec::new();

    for p in pools_raw.as_array().unwrap() {
        let id = p["id"].as_str().unwrap_or("").to_string();
        let address = p["address"].as_str().unwrap_or("").to_string();
        let pool_type = p["type"].as_str().unwrap_or("Unknown").to_string();
        let total_liquidity = p["dynamicData"]["totalLiquidity"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| p["dynamicData"]["totalLiquidity"].to_string());
        let swap_fee = p["dynamicData"]["swapFee"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| p["dynamicData"]["swapFee"].to_string());

        let tokens: Vec<PoolToken> = p["poolTokens"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|t| PoolToken {
                address: t["address"].as_str().unwrap_or("").to_string(),
                symbol: t["symbol"].as_str().unwrap_or("").to_string(),
                decimals: t["decimals"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| t["decimals"].to_string()),
                weight: t["weight"].as_str().map(|s| s.to_string()),
            })
            .collect();

        pools.push(PoolInfo {
            id,
            address,
            pool_type,
            total_liquidity_usd: total_liquidity,
            swap_fee,
            tokens,
        });
    }

    println!("{}", serde_json::to_string_pretty(&pools)?);
    Ok(())
}
