// Direct eth_call wrappers for Stader read operations
// Uses ethereum.publicnode.com — no rate limits per kb/onchainos/gotchas.md

use anyhow::Result;
use serde_json::{json, Value};

/// Generic eth_call — returns raw hex result string
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a single uint256 from a 32-byte hex result
pub fn decode_uint256(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return 0;
    }
    u128::from_str_radix(&clean[clean.len() - 32..], 16).unwrap_or(0)
}

/// Decode a bool from a 32-byte hex result
pub fn decode_bool(hex: &str) -> bool {
    decode_uint256(hex) != 0
}

/// Encode a single address parameter (32-byte padded)
pub fn encode_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Encode a single uint256 parameter (32-byte padded)
pub fn encode_uint256(val: u128) -> String {
    format!("{:064x}", val)
}

// =============================================================================
// StaderStakePoolsManager read functions
// =============================================================================

/// getExchangeRate() → 0xe6aa216c
/// Returns exchange rate: how many wei of ETH 1 ETHx is worth (scaled by 1e18)
pub async fn get_exchange_rate(rpc_url: &str, manager: &str) -> Result<u128> {
    let hex = eth_call(rpc_url, manager, "0xe6aa216c").await?;
    Ok(decode_uint256(&hex))
}

/// minDeposit() → 0x41b3d185
pub async fn get_min_deposit(rpc_url: &str, manager: &str) -> Result<u128> {
    let hex = eth_call(rpc_url, manager, "0x41b3d185").await?;
    Ok(decode_uint256(&hex))
}

/// maxDeposit() → 0x6083e59a
pub async fn get_max_deposit(rpc_url: &str, manager: &str) -> Result<u128> {
    let hex = eth_call(rpc_url, manager, "0x6083e59a").await?;
    Ok(decode_uint256(&hex))
}

/// totalAssets() → 0x01e1d114
pub async fn get_total_assets(rpc_url: &str, manager: &str) -> Result<u128> {
    let hex = eth_call(rpc_url, manager, "0x01e1d114").await?;
    Ok(decode_uint256(&hex))
}

/// isVaultHealthy() → 0xd5c9cfb0
pub async fn is_vault_healthy(rpc_url: &str, manager: &str) -> Result<bool> {
    let hex = eth_call(rpc_url, manager, "0xd5c9cfb0").await?;
    Ok(decode_bool(&hex))
}

/// previewDeposit(uint256 _assets) → 0xef8b30f7
/// Returns expected ETHx shares for given ETH amount
pub async fn preview_deposit(rpc_url: &str, manager: &str, eth_wei: u128) -> Result<u128> {
    let data = format!("0xef8b30f7{}", encode_uint256(eth_wei));
    let hex = eth_call(rpc_url, manager, &data).await?;
    Ok(decode_uint256(&hex))
}

/// convertToAssets(uint256 shares) → 0x07a2d13a
/// Returns ETH wei equivalent for given ETHx amount
pub async fn convert_to_assets(rpc_url: &str, manager: &str, shares: u128) -> Result<u128> {
    let data = format!("0x07a2d13a{}", encode_uint256(shares));
    let hex = eth_call(rpc_url, manager, &data).await?;
    Ok(decode_uint256(&hex))
}

// =============================================================================
// ETHx Token (ERC-20) read functions
// =============================================================================

/// balanceOf(address) → 0x70a08231
pub async fn ethx_balance_of(rpc_url: &str, token: &str, owner: &str) -> Result<u128> {
    let data = format!("0x70a08231{}", encode_address(owner));
    let hex = eth_call(rpc_url, token, &data).await?;
    Ok(decode_uint256(&hex))
}

/// allowance(address,address) → 0xdd62ed3e
pub async fn ethx_allowance(rpc_url: &str, token: &str, owner: &str, spender: &str) -> Result<u128> {
    let data = format!("0xdd62ed3e{}{}", encode_address(owner), encode_address(spender));
    let hex = eth_call(rpc_url, token, &data).await?;
    Ok(decode_uint256(&hex))
}

// =============================================================================
// UserWithdrawManager read functions
// =============================================================================

/// nextRequestId() → 0x6a84a985
#[allow(dead_code)]
pub async fn get_next_request_id(rpc_url: &str, mgr: &str) -> Result<u128> {
    let hex = eth_call(rpc_url, mgr, "0x6a84a985").await?;
    Ok(decode_uint256(&hex))
}

/// getRequestIdsByUser(address) → 0x7a99ab07
/// Returns ABI-encoded uint256[] — we decode manually
pub async fn get_request_ids_by_user(rpc_url: &str, mgr: &str, user: &str) -> Result<Vec<u128>> {
    let data = format!("0x7a99ab07{}", encode_address(user));
    let hex = eth_call(rpc_url, mgr, &data).await?;
    Ok(decode_uint256_array(&hex))
}

/// userWithdrawRequests(uint256 requestId) → 0x911f7acd
/// Returns tuple: (ethXAmount, ethExpected, ethFinalized, requestBlock, owner)
pub async fn get_withdraw_request(rpc_url: &str, mgr: &str, request_id: u128) -> Result<UserWithdrawInfo> {
    let data = format!("0x911f7acd{}", encode_uint256(request_id));
    let hex = eth_call(rpc_url, mgr, &data).await?;
    decode_withdraw_info(&hex, request_id)
}

// =============================================================================
// ABI decode helpers
// =============================================================================

/// Decode ABI-encoded uint256[] (dynamic array)
/// Format: offset(32) | length(32) | element[0](32) | ...
fn decode_uint256_array(hex: &str) -> Vec<u128> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return vec![];
    }
    // offset word at [0..64] — skip to length
    // length at [64..128]
    let length = u128::from_str_radix(&clean[64..128], 16).unwrap_or(0) as usize;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        let start = 128 + i * 64;
        let end = start + 64;
        if end > clean.len() {
            break;
        }
        let val = u128::from_str_radix(&clean[start..end], 16).unwrap_or(0);
        result.push(val);
    }
    result
}

#[derive(Debug, serde::Serialize)]
pub struct UserWithdrawInfo {
    pub request_id: u128,
    pub ethx_amount: String,       // ETHx locked (wei, as string for large u128)
    pub eth_expected: String,      // ETH expected
    pub eth_finalized: String,     // ETH claimable (0 if not finalized)
    pub request_block: u64,
    pub owner: String,
    pub is_finalized: bool,
}

/// Decode UserWithdrawInfo tuple from eth_call result
/// Tuple: (uint256 ethXAmount, uint256 ethExpected, uint256 ethFinalized, uint256 requestBlock, address owner)
fn decode_withdraw_info(hex: &str, request_id: u128) -> Result<UserWithdrawInfo> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 320 {
        anyhow::bail!("Unexpected response length for userWithdrawRequests");
    }
    let ethx_amount = u128::from_str_radix(&clean[0..64], 16).unwrap_or(0);
    let eth_expected = u128::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    let eth_finalized = u128::from_str_radix(&clean[128..192], 16).unwrap_or(0);
    let request_block = u128::from_str_radix(&clean[192..256], 16).unwrap_or(0) as u64;
    // owner is the last 40 hex chars of the 256-bit (64-char) slot
    let owner_hex = &clean[256..320];
    let owner = format!("0x{}", &owner_hex[24..]);

    Ok(UserWithdrawInfo {
        request_id,
        ethx_amount: ethx_amount.to_string(),
        eth_expected: eth_expected.to_string(),
        eth_finalized: eth_finalized.to_string(),
        request_block,
        owner,
        is_finalized: eth_finalized > 0,
    })
}

/// Format wei as ETH string (18 decimals)
pub fn format_eth(wei: u128) -> String {
    let integer = wei / 1_000_000_000_000_000_000;
    let frac = (wei % 1_000_000_000_000_000_000) / 10_000_000_000_000; // 5 decimals
    format!("{}.{:05}", integer, frac)
}
