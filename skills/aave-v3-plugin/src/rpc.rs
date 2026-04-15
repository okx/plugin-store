use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Raw JSON-RPC request/response
#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: Value,
    id: u64,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<String>,
    error: Option<Value>,
}

/// Poll eth_getTransactionReceipt until the tx is mined (or timeout).
/// Returns true if the tx succeeded (status=0x1), false if reverted, error if timed out.
pub async fn wait_for_tx(rpc_url: &str, tx_hash: &str) -> anyhow::Result<bool> {
    use std::time::{Duration, Instant};
    let client = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(60);

    loop {
        if Instant::now() > deadline {
            anyhow::bail!("Timeout waiting for tx {} to be mined", tx_hash);
        }

        let req = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });

        match client.post(rpc_url).json(&req).send().await {
            Ok(resp) => {
                if let Ok(body) = resp.json::<Value>().await {
                    let receipt = &body["result"];
                    if !receipt.is_null() {
                        let status = receipt["status"].as_str().unwrap_or("0x1");
                        return Ok(status == "0x1");
                    }
                }
            }
            Err(_) => {}
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

/// Perform a raw eth_call against the given RPC endpoint.
/// `to` and `data` are hex strings (0x-prefixed).
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let req = RpcRequest {
        jsonrpc: "2.0",
        method: "eth_call",
        params: json!([
            { "to": to, "data": data },
            "latest"
        ]),
        id: 1,
    };
    let resp: RpcResponse = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call response parse failed")?;

    if let Some(err) = resp.error {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    resp.result
        .ok_or_else(|| anyhow::anyhow!("eth_call returned null result"))
}

/// Resolve the Pool address by calling PoolAddressesProvider.getPool()
/// Function selector: getPool() -> 0x026b1d5f
/// Verified on-chain against Aave V3 deployments on Ethereum, Base, Polygon, Arbitrum.
/// Note: 0x0c2c3d97 (often cited as getPool() selector) is incorrect for the actual
/// deployed PoolAddressesProvider — 0x026b1d5f is the correct observed selector.
pub async fn get_pool(provider_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    // getPool() selector — verified empirically against live Aave V3 deployments
    let data = "0x026b1d5f";
    let hex_result = eth_call(rpc_url, provider_addr, data).await?;
    // Result is a 32-byte ABI-encoded address (left-padded with zeros)
    let addr = decode_address_result(&hex_result)?;
    Ok(addr)
}

#[allow(dead_code)]
/// Resolve the PoolDataProvider address by calling PoolAddressesProvider.getPoolDataProvider()
/// Function selector: getPoolDataProvider() -> 0x0e67178c
/// Verified on-chain against Aave V3 Base deployment.
pub async fn get_pool_data_provider(provider_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    // getPoolDataProvider() selector — verified empirically against live Aave V3 deployments
    let data = "0x0e67178c";
    let hex_result = eth_call(rpc_url, provider_addr, data).await?;
    let addr = decode_address_result(&hex_result)?;
    Ok(addr)
}

/// Account data returned by Pool.getUserAccountData(address)
#[derive(Debug, Clone)]
pub struct UserAccountData {
    /// Total collateral in USD base units (8 decimals)
    pub total_collateral_base: u128,
    /// Total debt in USD base units (8 decimals)
    pub total_debt_base: u128,
    /// Available borrows in USD base units (8 decimals)
    pub available_borrows_base: u128,
    /// Current liquidation threshold (basis points, e.g. 8250 = 82.5%)
    pub current_liquidation_threshold: u128,
    /// LTV (basis points)
    pub ltv: u128,
    /// Health factor scaled 1e18 (< 1e18 = liquidatable)
    pub health_factor: u128,
}

impl UserAccountData {
    /// Returns health factor as a human-readable f64
    pub fn health_factor_f64(&self) -> f64 {
        self.health_factor as f64 / 1e18
    }

    /// Returns health factor status string
    pub fn health_factor_status(&self) -> &'static str {
        let hf = self.health_factor_f64();
        if hf >= 1.1 {
            "safe"
        } else if hf >= 1.05 {
            "warning"
        } else {
            "danger"
        }
    }

    /// Returns total collateral in USD as f64
    pub fn total_collateral_usd(&self) -> f64 {
        self.total_collateral_base as f64 / 1e8
    }

    /// Returns total debt in USD as f64
    pub fn total_debt_usd(&self) -> f64 {
        self.total_debt_base as f64 / 1e8
    }

    /// Returns available borrows in USD as f64
    pub fn available_borrows_usd(&self) -> f64 {
        self.available_borrows_base as f64 / 1e8
    }
}

/// Call Pool.getUserAccountData(address user)
/// Function selector: 0xbf92857c
pub async fn get_user_account_data(
    pool_addr: &str,
    user_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<UserAccountData> {
    // Encode: selector (4 bytes) + address (32 bytes, left-padded)
    let addr_bytes = parse_address(user_addr)?;
    let mut data = hex::decode("bf92857c")?;
    // Pad address to 32 bytes
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);

    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, pool_addr, &data_hex).await?;

    // Result: 6 x uint256 packed (each 32 bytes = 64 hex chars)
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 * 6 {
        anyhow::bail!(
            "getUserAccountData: short response ({} hex chars, expected {})",
            raw.len(),
            64 * 6
        );
    }

    Ok(UserAccountData {
        total_collateral_base: decode_u128_at(raw, 0)?,
        total_debt_base: decode_u128_at(raw, 1)?,
        available_borrows_base: decode_u128_at(raw, 2)?,
        current_liquidation_threshold: decode_u128_at(raw, 3)?,
        ltv: decode_u128_at(raw, 4)?,
        health_factor: decode_u128_at(raw, 5)?,
    })
}

/// Get ERC-20 token balance: token.balanceOf(account)
/// Function selector: balanceOf(address) -> 0x70a08231
pub async fn get_erc20_balance(
    token_addr: &str,
    account: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner = parse_address(account)?;
    let mut data = hex::decode("70a08231")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&owner);

    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        anyhow::bail!("balanceOf: short response");
    }
    decode_u128_at(raw, 0)
}

/// Check ERC-20 allowance: token.allowance(owner, spender)
/// Function selector: allowance(address,address) -> 0xdd62ed3e
pub async fn get_allowance(
    token_addr: &str,
    owner_addr: &str,
    spender_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner = parse_address(owner_addr)?;
    let spender = parse_address(spender_addr)?;

    let mut data = hex::decode("dd62ed3e")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&owner);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&spender);

    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        anyhow::bail!("allowance: short response");
    }
    decode_u128_at(raw, 0)
}

/// Get ERC-20 token symbol: token.symbol()
/// Function selector: symbol() -> 0x95d89b41
pub async fn get_erc20_symbol(token_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex_result = eth_call(rpc_url, token_addr, "0x95d89b41").await?;
    let raw = strip_0x(&hex_result);
    // ABI-encoded string: offset(32) + length(32) + data(padded)
    if raw.len() < 128 {
        return Ok(String::new());
    }
    let len = usize::from_str_radix(&raw[64..128], 16).unwrap_or(0);
    if len == 0 || raw.len() < 128 + len * 2 {
        return Ok(String::new());
    }
    let bytes = hex::decode(&raw[128..128 + len * 2]).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Get native ETH balance via eth_getBalance.
pub async fn get_eth_balance(account: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let client = reqwest::Client::new();
    let req = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [account, "latest"],
        "id": 1
    });
    let resp: RpcResponse = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .context("eth_getBalance HTTP request failed")?
        .json()
        .await
        .context("eth_getBalance response parse failed")?;
    if let Some(err) = resp.error {
        anyhow::bail!("eth_getBalance RPC error: {}", err);
    }
    let hex_str = resp.result.ok_or_else(|| anyhow::anyhow!("eth_getBalance returned null"))?;
    let raw = strip_0x(&hex_str);
    u128::from_str_radix(raw, 16).context("eth_getBalance: hex parse error")
}

/// Get the aToken address for an asset via IPoolDataProvider.getReserveTokensAddresses(asset).
/// Selector 0xd2493b6c — verified against Aave V3 PoolDataProvider.
/// Returns the aTokenAddress (first of the three returned addresses).
pub async fn get_atoken_address(
    data_provider: &str,
    asset: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let asset_bytes = parse_address(asset)?;
    let mut data = hex::decode("d2493b6c")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&asset_bytes);
    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, data_provider, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    // Returns 3 x address (each 32 bytes = 64 hex chars), total 192 hex chars minimum
    if raw.len() < 192 {
        anyhow::bail!("getReserveTokensAddresses: short response ({} hex chars)", raw.len());
    }
    // First slot (bytes 0..64) = aTokenAddress
    decode_address_result(&format!("0x{}", &raw[0..64]))
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn strip_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

fn decode_address_result(hex_result: &str) -> anyhow::Result<String> {
    let raw = strip_0x(hex_result);
    if raw.len() < 64 {
        anyhow::bail!("decode_address_result: short result '{}'", hex_result);
    }
    // Last 40 hex chars = 20 byte address
    let addr_hex = &raw[raw.len() - 40..];
    Ok(format!("0x{}", addr_hex))
}

fn parse_address(addr: &str) -> anyhow::Result<[u8; 20]> {
    let clean = strip_0x(addr);
    if clean.len() != 40 {
        anyhow::bail!("Invalid address (must be 20 bytes / 40 hex chars): {}", addr);
    }
    let bytes = hex::decode(clean).context("Invalid hex address")?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_u128_at(raw: &str, slot: usize) -> anyhow::Result<u128> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_u128_at: slot {} out of range (raw len {})", slot, raw.len());
    }
    let slot_hex = &raw[start..end];
    // u256 may exceed u128 — take lower 32 hex chars (16 bytes)
    let low32 = &slot_hex[32..64];
    let val = u128::from_str_radix(low32, 16)
        .with_context(|| format!("decode_u128_at: invalid hex '{}'", low32))?;
    Ok(val)
}
