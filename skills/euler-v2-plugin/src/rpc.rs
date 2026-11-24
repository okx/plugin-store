/// Direct RPC eth_call wrapper for Euler v2 plugin.
///
/// Used for read-only vault queries (`balanceOf`, `previewRedeem`, `debtOf`, etc.)
/// where we don't want to spawn an `onchainos` subprocess per call.
///
/// All calldata uses raw 0x-prefixed hex; ABI encoding is done at the call site
/// using the helpers in `calldata.rs`.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::Urls;

/// Make a single eth_call against the chain's public RPC. Returns the result hex
/// string (e.g. `"0x000...000064"` for uint256 = 100). Caller decodes.
pub async fn eth_call(chain_id: u64, to: &str, data: &str) -> Result<String> {
    let rpc = Urls::rpc_for_chain(chain_id)
        .ok_or_else(|| anyhow::anyhow!("No RPC URL configured for chain {}", chain_id))?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": data }, "latest"],
        "id": 1
    });
    let resp = reqwest::Client::new()
        .post(&rpc)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("eth_call to {} failed (chain {})", to, chain_id))?;
    let v: Value = resp.json().await
        .context("Parsing eth_call response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("RPC error on chain {}: {}", chain_id, err);
    }
    Ok(v["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a uint256 hex result into u128. If the value > u128::MAX, returns u128::MAX
/// (saturating). Returns 0 for empty/zero hex.
pub fn parse_uint256_to_u128(hex: &str) -> u128 {
    let stripped = hex.trim_start_matches("0x");
    if stripped.is_empty() || stripped.chars().all(|c| c == '0') {
        return 0;
    }
    u128::from_str_radix(stripped, 16).unwrap_or(u128::MAX)
}

/// Pad an address to 32 bytes hex (no 0x prefix). Used for ABI-encoded args.
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean.to_lowercase())
}

/// Standard ERC-20 / ERC-4626 `balanceOf(address)` selector.
pub const SELECTOR_BALANCE_OF: &str = "70a08231";

/// ERC-4626 `previewRedeem(uint256 shares) -> assets`.
pub const SELECTOR_PREVIEW_REDEEM: &str = "4cdad506";

/// EVK `debtOf(address) -> uint256`.
/// keccak256("debtOf(address)")[:4] = 0xd283e75f (verified at runtime by tests)
pub const SELECTOR_DEBT_OF: &str = "d283e75f";

/// Build calldata for a function taking a single `address` argument.
pub fn build_address_call(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, pad_address(addr))
}

/// Build calldata for `previewRedeem(uint256 shares)`.
pub fn build_preview_redeem(shares: u128) -> String {
    format!("0x{}{:064x}", SELECTOR_PREVIEW_REDEEM, shares)
}

/// Query the native gas balance (ETH on Ethereum/Base/Arbitrum, MATIC on Polygon)
/// for `addr` on the given chain. Returns the balance in wei as u128.
/// Used by write commands' gas pre-flight per [GAS-001].
pub async fn eth_get_balance_wei(chain_id: u64, addr: &str) -> Result<u128> {
    let rpc = Urls::rpc_for_chain(chain_id)
        .ok_or_else(|| anyhow::anyhow!("No RPC URL configured for chain {}", chain_id))?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [addr, "latest"],
        "id": 1
    });
    let resp = reqwest::Client::new()
        .post(&rpc)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("eth_getBalance for {} on chain {} failed", addr, chain_id))?;
    let v: Value = resp.json().await
        .context("Parsing eth_getBalance response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("RPC error on chain {}: {}", chain_id, err);
    }
    Ok(parse_uint256_to_u128(v["result"].as_str().unwrap_or("0x")))
}

/// Convert wei to ETH (f64). Used purely for human-readable output / error messages.
pub fn wei_to_eth(wei: u128) -> f64 {
    wei as f64 / 1e18
}

/// Estimate gas cost for a call: gas_limit × gas_price (fetched from RPC).
/// Returns wei as u128. Used to bail with a precise INSUFFICIENT_ETH_GAS error
/// before users sign anything.
pub async fn estimate_native_gas_cost_wei(chain_id: u64, gas_limit: u64) -> Result<u128> {
    let rpc = Urls::rpc_for_chain(chain_id)
        .ok_or_else(|| anyhow::anyhow!("No RPC URL configured for chain {}", chain_id))?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_gasPrice",
        "params": [],
        "id": 1
    });
    let resp = reqwest::Client::new()
        .post(&rpc)
        .json(&body)
        .send()
        .await
        .context("eth_gasPrice request failed")?;
    let v: Value = resp.json().await.context("Parsing eth_gasPrice response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("RPC error on chain {}: {}", chain_id, err);
    }
    let gas_price = parse_uint256_to_u128(v["result"].as_str().unwrap_or("0x"));
    // 20% buffer on top of current gas price to absorb fluctuations between estimate and broadcast.
    let buffered = gas_price.saturating_mul(120) / 100;
    Ok(buffered.saturating_mul(gas_limit as u128))
}
