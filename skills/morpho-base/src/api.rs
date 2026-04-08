use anyhow::Context;
use serde::{Deserialize, Deserializer};
use crate::config::GRAPHQL_URL;

/// Deserialize a field that may be a JSON number or string into Option<String>.
fn deser_number_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    Ok(v.map(|val| match val {
        serde_json::Value::String(s) => s,
        other => other.to_string(),
    }))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketParams {
    pub loan_token: String,
    pub collateral_token: String,
    pub oracle: String,
    pub irm: String,
    pub lltv: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketState {
    pub supply_apy: Option<f64>,
    pub borrow_apy: Option<f64>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub supply_assets: Option<String>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub borrow_assets: Option<String>,
    pub utilization: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    pub unique_key: String,
    pub loan_asset: Option<Asset>,
    pub collateral_asset: Option<Asset>,
    pub oracle_address: Option<String>,
    pub irm_address: Option<String>,
    pub lltv: Option<String>,
    pub state: Option<MarketState>,
    pub params: Option<MarketParams>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub address: String,
    pub symbol: String,
    pub decimals: Option<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionState {
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub supply_assets: Option<String>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub borrow_assets: Option<String>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub collateral: Option<String>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub supply_shares: Option<String>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub borrow_shares: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPosition {
    pub market: Market,
    pub state: PositionState,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultState {
    pub apy: Option<f64>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub total_assets: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vault {
    pub address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub asset: Option<Asset>,
    pub state: Option<VaultState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultPosition {
    pub vault: Vault,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub assets: Option<String>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub shares: Option<String>,
}

async fn graphql_query(query: &str, variables: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "query": query, "variables": variables });
    let resp: serde_json::Value = client
        .post(GRAPHQL_URL)
        .json(&body)
        .send()
        .await
        .context("GraphQL request failed")?
        .json()
        .await
        .context("GraphQL response parse failed")?;

    if let Some(errors) = resp.get("errors") {
        anyhow::bail!("GraphQL errors: {}", errors);
    }
    Ok(resp)
}

/// Fetch full market details (including MarketParams) for a given market uniqueKey.
pub async fn get_market(unique_key: &str, chain_id: u64) -> anyhow::Result<Market> {
    let query = r#"
        query GetMarket($uniqueKey: String!, $chainId: Int!) {
            marketByUniqueKey(uniqueKey: $uniqueKey, chainId: $chainId) {
                uniqueKey
                loanAsset { address symbol decimals }
                collateralAsset { address symbol decimals }
                oracleAddress
                irmAddress
                lltv
                state {
                    supplyApy
                    borrowApy
                    supplyAssets
                    borrowAssets
                    utilization
                }
            }
        }
    "#;
    let vars = serde_json::json!({ "uniqueKey": unique_key, "chainId": chain_id });
    let resp = graphql_query(query, vars).await?;
    let market: Market = serde_json::from_value(resp["data"]["marketByUniqueKey"].clone())
        .context("Failed to parse market from GraphQL response")?;
    Ok(market)
}

/// Fetch all markets for a chain, optionally filtered by loan asset symbol.
pub async fn list_markets(chain_id: u64, asset_filter: Option<&str>) -> anyhow::Result<Vec<Market>> {
    let query = r#"
        query ListMarkets($chainId: Int!, $first: Int!) {
            markets(where: { chainId_in: [$chainId] }, first: $first) {
                items {
                    uniqueKey
                    loanAsset { address symbol decimals }
                    collateralAsset { address symbol decimals }
                    oracleAddress
                    irmAddress
                    lltv
                    state {
                        supplyApy
                        borrowApy
                        supplyAssets
                        borrowAssets
                        utilization
                    }
                }
            }
        }
    "#;
    let vars = serde_json::json!({ "chainId": chain_id, "first": 50 });
    let resp = graphql_query(query, vars).await?;

    let items = resp["data"]["markets"]["items"]
        .as_array()
        .context("Missing markets items")?;

    let mut markets: Vec<Market> = items
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    if let Some(filter) = asset_filter {
        let filter_lower = filter.to_lowercase();
        markets.retain(|m| {
            m.loan_asset
                .as_ref()
                .map(|a| a.symbol.to_lowercase().contains(&filter_lower))
                .unwrap_or(false)
        });
    }

    Ok(markets)
}

/// Fetch user's market positions.
pub async fn get_user_positions(user: &str, chain_id: u64) -> anyhow::Result<Vec<MarketPosition>> {
    let query = r#"
        query UserPositions($address: String!, $chainId: Int!) {
            marketPositions(where: { userAddress_in: [$address], chainId_in: [$chainId] }) {
                items {
                    market {
                        uniqueKey
                        loanAsset { address symbol decimals }
                        collateralAsset { address symbol decimals }
                        lltv
                    }
                    state {
                        supplyAssets
                        borrowAssets
                        collateral
                        supplyShares
                        borrowShares
                    }
                }
            }
        }
    "#;
    let vars = serde_json::json!({ "address": user, "chainId": chain_id });
    let resp = graphql_query(query, vars).await?;

    let items = resp["data"]["marketPositions"]["items"]
        .as_array()
        .context("Missing marketPositions items")?;

    let positions: Vec<MarketPosition> = items
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    Ok(positions)
}

/// Fetch user's vault positions.
pub async fn get_vault_positions(user: &str, chain_id: u64) -> anyhow::Result<Vec<VaultPosition>> {
    let query = r#"
        query VaultPositions($address: String!, $chainId: Int!) {
            vaultPositions(where: { userAddress_in: [$address], chainId_in: [$chainId] }) {
                items {
                    vault {
                        address
                        name
                        symbol
                        asset { address symbol decimals }
                        state { apy totalAssets }
                    }
                    assets
                    shares
                }
            }
        }
    "#;
    let vars = serde_json::json!({ "address": user, "chainId": chain_id });
    let resp = graphql_query(query, vars).await?;

    let items = resp["data"]["vaultPositions"]["items"]
        .as_array()
        .context("Missing vaultPositions items")?;

    let positions: Vec<VaultPosition> = items
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    Ok(positions)
}

/// List MetaMorpho vaults, optionally filtered by asset symbol.
pub async fn list_vaults(chain_id: u64, asset_filter: Option<&str>) -> anyhow::Result<Vec<Vault>> {
    let query = r#"
        query ListVaults($chainId: Int!, $first: Int!) {
            vaults(where: { chainId_in: [$chainId] }, first: $first) {
                items {
                    address
                    name
                    symbol
                    asset { address symbol decimals }
                    state { apy totalAssets }
                }
            }
        }
    "#;
    let vars = serde_json::json!({ "chainId": chain_id, "first": 50 });
    let resp = graphql_query(query, vars).await?;

    let items = resp["data"]["vaults"]["items"]
        .as_array()
        .context("Missing vaults items")?;

    let mut vaults: Vec<Vault> = items
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    if let Some(filter) = asset_filter {
        let filter_lower = filter.to_lowercase();
        vaults.retain(|v| {
            v.asset
                .as_ref()
                .map(|a| a.symbol.to_lowercase().contains(&filter_lower))
                .unwrap_or(false)
        });
    }

    Ok(vaults)
}

/// Build MarketParams from a fetched market.
pub fn build_market_params(market: &Market) -> anyhow::Result<crate::calldata::MarketParamsData> {
    let loan_token = market
        .loan_asset
        .as_ref()
        .map(|a| a.address.clone())
        .unwrap_or_default();
    let collateral_token = market
        .collateral_asset
        .as_ref()
        .map(|a| a.address.clone())
        .unwrap_or_default();
    let oracle = market.oracle_address.clone().unwrap_or_default();
    let irm = market.irm_address.clone().unwrap_or_default();
    let lltv_str = market.lltv.clone().unwrap_or_else(|| "0".to_string());
    let lltv: u128 = lltv_str.parse().unwrap_or(0);

    Ok(crate::calldata::MarketParamsData {
        loan_token,
        collateral_token,
        oracle,
        irm,
        lltv,
    })
}
