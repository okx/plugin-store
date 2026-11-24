use crate::config::GRAPHQL_URL;
use anyhow::Context;
use serde::{Deserialize, Deserializer};

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
pub struct Oracle {
    pub address: String,
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
    pub market_id: String,
    pub loan_asset: Option<Asset>,
    pub collateral_asset: Option<Asset>,
    pub oracle: Option<Oracle>,
    pub irm_address: Option<String>,
    pub lltv: Option<String>,
    pub state: Option<MarketState>,
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
pub struct VaultPositionState {
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub assets: Option<String>,
    #[serde(deserialize_with = "deser_number_or_string", default)]
    pub shares: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultPosition {
    pub vault: Vault,
    pub state: VaultPositionState,
}

async fn graphql_query(
    query: &str,
    variables: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
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

    ensure_graphql_success(&resp)?;
    Ok(resp)
}

fn ensure_graphql_success(resp: &serde_json::Value) -> anyhow::Result<()> {
    if let Some(errors) = resp.get("errors").filter(|errors| !errors.is_null()) {
        anyhow::bail!("GraphQL errors: {}", errors);
    }
    Ok(())
}

fn parse_market_response(resp: &serde_json::Value) -> anyhow::Result<Market> {
    let value = resp["data"]["marketById"].clone();
    if value.is_null() {
        anyhow::bail!("Morpho market detail response did not contain data.marketById");
    }
    serde_json::from_value(value).context("Failed to parse market detail from GraphQL response")
}

const GET_MARKET_QUERY: &str = r#"
    query GetMarket($marketId: String!, $chainId: Int!) {
        marketById(marketId: $marketId, chainId: $chainId) {
            marketId
            loanAsset { address symbol decimals }
            collateralAsset { address symbol decimals }
            oracle { address }
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

/// Fetch full market details (including the fields required for MarketParams)
/// for a given Morpho Blue market ID.
pub async fn get_market(market_id: &str, chain_id: u64) -> anyhow::Result<Market> {
    let vars = serde_json::json!({ "marketId": market_id, "chainId": chain_id });
    let resp = graphql_query(GET_MARKET_QUERY, vars).await?;
    parse_market_response(&resp)
}

/// Fetch all markets for a chain, optionally filtered by loan asset symbol.
pub async fn list_markets(
    chain_id: u64,
    asset_filter: Option<&str>,
) -> anyhow::Result<Vec<Market>> {
    let query = r#"
        query ListMarkets($chainId: Int!, $first: Int!) {
            markets(where: { chainId_in: [$chainId] }, first: $first) {
                items {
                    marketId
                    loanAsset { address symbol decimals }
                    collateralAsset { address symbol decimals }
                    oracle { address }
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
                        marketId
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
                    state {
                        assets
                        shares
                    }
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
        .map(|a| a.address.trim())
        .filter(|address| !address.is_empty())
        .context("Market detail is missing loanAsset.address")?
        .to_string();
    let collateral_token = market
        .collateral_asset
        .as_ref()
        .map(|a| a.address.trim())
        .filter(|address| !address.is_empty())
        .context("Market detail is missing collateralAsset.address")?
        .to_string();
    let oracle = market
        .oracle
        .as_ref()
        .map(|oracle| oracle.address.trim())
        .filter(|address| !address.is_empty())
        .context("Market detail is missing oracle.address")?
        .to_string();
    let irm = market
        .irm_address
        .as_deref()
        .map(str::trim)
        .filter(|address| !address.is_empty())
        .context("Market detail is missing irmAddress")?
        .to_string();
    let lltv_str = market
        .lltv
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .context("Market detail is missing lltv")?;
    let lltv: u128 = lltv_str
        .parse()
        .with_context(|| format!("Market detail contains invalid lltv '{}'", lltv_str))?;
    if lltv == 0 {
        anyhow::bail!("Market detail contains zero lltv");
    }

    Ok(crate::calldata::MarketParamsData {
        loan_token,
        collateral_token,
        oracle,
        irm,
        lltv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_market_fixture() -> serde_json::Value {
        serde_json::json!({
            "data": {
                "marketById": {
                    "marketId": "0xmarket",
                    "loanAsset": { "address": "0xloan", "symbol": "USDC", "decimals": 6 },
                    "collateralAsset": { "address": "0xcollateral", "symbol": "WETH", "decimals": 18 },
                    "oracle": { "address": "0xoracle" },
                    "irmAddress": "0xirm",
                    "lltv": "860000000000000000",
                    "state": null
                }
            }
        })
    }

    #[test]
    fn current_market_schema_builds_complete_market_params() {
        assert!(GET_MARKET_QUERY.contains("marketById(marketId:"));
        assert!(GET_MARKET_QUERY.contains("oracle { address }"));
        assert!(!GET_MARKET_QUERY.contains("marketByUniqueKey"));
        let market = parse_market_response(&current_market_fixture()).unwrap();
        assert_eq!(market.market_id, "0xmarket");

        let params = build_market_params(&market).unwrap();
        assert_eq!(params.loan_token, "0xloan");
        assert_eq!(params.collateral_token, "0xcollateral");
        assert_eq!(params.oracle, "0xoracle");
        assert_eq!(params.irm, "0xirm");
        assert_eq!(params.lltv, 860_000_000_000_000_000);
    }

    #[test]
    fn market_detail_failure_is_distinct_from_a_position_response() {
        let position_only = serde_json::json!({
            "data": { "marketPositions": { "items": [] } }
        });
        let err = parse_market_response(&position_only).unwrap_err();
        assert!(
            err.to_string().contains("market detail") && err.to_string().contains("marketById"),
            "got: {err:#}"
        );
    }

    #[test]
    fn missing_required_market_param_is_rejected() {
        let mut fixture = current_market_fixture();
        fixture["data"]["marketById"]["oracle"] = serde_json::Value::Null;
        let market = parse_market_response(&fixture).unwrap();
        let err = build_market_params(&market).unwrap_err();
        assert!(err.to_string().contains("oracle.address"), "got: {err:#}");
    }

    #[test]
    fn graphql_unknown_field_error_keeps_server_detail() {
        let response = serde_json::json!({
            "errors": [{
                "message": "Cannot query field \"marketByUniqueKey\" on type \"Query\"."
            }]
        });
        let rendered = ensure_graphql_success(&response).unwrap_err().to_string();
        assert!(rendered.contains("marketByUniqueKey"));
        assert!(rendered.contains("Cannot query field"));
    }

    #[test]
    fn current_vault_position_schema_reads_balances_from_state() {
        let fixture = serde_json::json!({
            "vault": {
                "address": "0xvault",
                "name": "Test Vault",
                "symbol": "tvUSDC",
                "asset": { "address": "0xloan", "symbol": "USDC", "decimals": 6 },
                "state": { "apy": 0.05, "totalAssets": "1000000" }
            },
            "state": {
                "assets": "123456",
                "shares": "120000"
            }
        });
        let position: VaultPosition = serde_json::from_value(fixture).unwrap();
        assert_eq!(position.state.assets.as_deref(), Some("123456"));
        assert_eq!(position.state.shares.as_deref(), Some("120000"));
    }
}
