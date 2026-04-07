use crate::config;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

/// Build HTTP client with proxy support.
pub fn build_client() -> Client {
    let mut builder = Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

#[derive(Debug, Deserialize)]
pub struct VaultInfo {
    pub id: String,
    #[serde(rename = "isWhitelisted")]
    pub is_whitelisted: bool,
    pub asset: TokenInfo,
    #[serde(rename = "yieldToken")]
    pub yield_token: TokenInfo,
}

#[derive(Debug, Deserialize)]
pub struct TokenInfo {
    pub id: String,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub decimals: Option<i64>,
    #[serde(rename = "tokenAddress", default)]
    pub token_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AccountBalance {
    pub id: String,
    pub token: BalanceToken,
    pub current: BalanceSnapshot,
    #[serde(rename = "lendingRouter", default)]
    pub lending_router: Option<RouterInfo>,
}

#[derive(Debug, Deserialize)]
pub struct BalanceToken {
    pub id: String,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(rename = "tokenType")]
    pub token_type: String,
    #[serde(rename = "vaultAddress", default)]
    pub vault_address: Option<VaultRef>,
}

#[derive(Debug, Deserialize)]
pub struct VaultRef {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct BalanceSnapshot {
    #[serde(rename = "currentBalance", default)]
    pub current_balance: Option<String>,
    #[serde(rename = "currentProfitAndLossAtSnapshot", default)]
    pub pnl: Option<String>,
    #[serde(rename = "impliedFixedRate", default)]
    pub implied_fixed_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RouterInfo {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// Query all whitelisted vaults from the Notional Exponent subgraph.
pub async fn get_vaults() -> anyhow::Result<Vec<VaultInfo>> {
    let client = build_client();
    let query = r#"
    {
      vaults(where: { isWhitelisted: true }) {
        id
        isWhitelisted
        asset { id symbol decimals tokenAddress }
        yieldToken { id symbol decimals tokenAddress }
      }
    }
    "#;

    let resp = client
        .post(config::SUBGRAPH_URL)
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let vaults: Vec<VaultInfo> = serde_json::from_value(
        resp["data"]["vaults"].clone(),
    )?;
    Ok(vaults)
}

/// Query account positions from the subgraph.
pub async fn get_account_balances(wallet: &str) -> anyhow::Result<Vec<AccountBalance>> {
    let client = build_client();
    let query = format!(
        r#"
    {{
      account(id: "{}") {{
        balances {{
          id
          token {{ id symbol tokenType vaultAddress {{ id }} }}
          current {{ currentBalance currentProfitAndLossAtSnapshot impliedFixedRate }}
          lendingRouter {{ id name }}
        }}
      }}
    }}
    "#,
        wallet.to_lowercase()
    );

    let resp = client
        .post(config::SUBGRAPH_URL)
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let account = &resp["data"]["account"];
    if account.is_null() {
        return Ok(vec![]);
    }

    let balances: Vec<AccountBalance> =
        serde_json::from_value(account["balances"].clone()).unwrap_or_default();
    Ok(balances)
}

/// eth_call to Ethereum RPC.
pub async fn eth_call(to: &str, data: &str) -> anyhow::Result<String> {
    let client = build_client();
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });
    let resp = client
        .post(config::ETHEREUM_RPC)
        .json(&payload)
        .send()
        .await?
        .json::<Value>()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    let result = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(result)
}

/// Get health factor for a user/vault pair.
/// Returns health factor as u128 (divide by 1e18 for percentage).
/// Returns 0 if no position exists.
pub async fn get_health_factor(user: &str, vault: &str) -> anyhow::Result<u128> {
    // healthFactor(address user, address vault) selector = 0x576f5c40
    let user_padded = format!("{:0>64}", &user[2..]);
    let vault_padded = format!("{:0>64}", &vault[2..]);
    let data = format!("0x576f5c40{}{}", user_padded, vault_padded);
    let result = eth_call(config::MORPHO_LENDING_ROUTER, &data).await?;
    if result == "0x" || result.len() < 66 {
        return Ok(0);
    }
    let hex = result.trim_start_matches("0x");
    let val = u128::from_str_radix(&hex[..32.min(hex.len())], 16).unwrap_or(0);
    Ok(val)
}

/// Get collateral balance for a user/vault pair.
/// Returns balance in vault share units.
pub async fn get_collateral_balance(user: &str, vault: &str) -> anyhow::Result<u128> {
    // balanceOfCollateral(address,address) = 0xda3a855f
    let user_padded = format!("{:0>64}", &user[2..]);
    let vault_padded = format!("{:0>64}", &vault[2..]);
    let data = format!("0xda3a855f{}{}", user_padded, vault_padded);
    let result = eth_call(config::MORPHO_LENDING_ROUTER, &data).await?;
    if result == "0x" || result.len() < 66 {
        return Ok(0);
    }
    let hex = result.trim_start_matches("0x");
    // Take last 32 hex chars of the uint256 to avoid overflow
    let start = hex.len().saturating_sub(32);
    let val = u128::from_str_radix(&hex[start..], 16).unwrap_or(0);
    Ok(val)
}

/// Resolve vault name from known addresses.
pub fn vault_name(addr: &str) -> &'static str {
    match addr.to_lowercase().as_str() {
        x if x == config::VAULT_SUSDE => "sUSDe Staking (USDC)",
        x if x == config::VAULT_MAPOLLO => "mAPOLLO Leveraged (USDC)",
        x if x == config::VAULT_MHYPER => "mHYPER Leveraged (USDC)",
        x if x == config::VAULT_WEETH => "weETH Leveraged (WETH)",
        x if x == config::VAULT_PT_SUSDE => "Pendle PT-sUSDE (USDC)",
        x if x == config::VAULT_LIUSD => "liUSD-4w Leveraged (USDC)",
        x if x == config::VAULT_OETH => "Convex OETH/WETH (WETH)",
        x if x == config::VAULT_MHYPER2 => "mHYPER Leveraged 2 (USDC)",
        _ => "Unknown Vault",
    }
}
