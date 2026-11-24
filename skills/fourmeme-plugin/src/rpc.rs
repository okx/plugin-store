/// Direct RPC eth_call wrappers for fourmeme-plugin.
///
/// All read-only helper3 / ERC-20 reads bypass `onchainos` to avoid spawning a
/// subprocess per call. Calldata is built in `calldata.rs`; this module is just
/// the JSON-RPC transport.

use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::Urls;

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
    let v: Value = resp.json().await.context("Parsing eth_call response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("RPC error on chain {}: {}", chain_id, err);
    }
    Ok(v["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode uint256 hex → u128 (saturating). Returns 0 for empty/zero.
pub fn parse_uint256_to_u128(hex: &str) -> u128 {
    let stripped = hex.trim_start_matches("0x");
    if stripped.is_empty() || stripped.chars().all(|c| c == '0') {
        return 0;
    }
    u128::from_str_radix(stripped, 16).unwrap_or(u128::MAX)
}

/// Decode an address word (right-aligned in 32 bytes).
pub fn parse_address(word: &str) -> String {
    let w = word.trim_start_matches("0x");
    if w.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    format!("0x{}", &w[w.len() - 40..])
}

/// Pad an address to 32 bytes hex (lowercase, no 0x prefix).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean.to_lowercase())
}

pub fn build_address_call(selector: &str, addr: &str) -> String {
    format!("0x{}{}", selector, pad_address(addr))
}

// ─── Native gas balance / pre-flight ───────────────────────────────────────────

/// Native BNB balance in wei. Used by GAS-001 pre-flight + INSUFFICIENT_BNB checks.
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
        .with_context(|| format!("eth_getBalance failed for {} on chain {}", addr, chain_id))?;
    let v: Value = resp.json().await.context("Parsing eth_getBalance response")?;
    if let Some(err) = v.get("error") {
        anyhow::bail!("RPC error on chain {}: {}", chain_id, err);
    }
    Ok(parse_uint256_to_u128(v["result"].as_str().unwrap_or("0x")))
}

pub fn wei_to_bnb(wei: u128) -> f64 {
    wei as f64 / 1e18
}

/// gas_price × gas_limit (with 20 % buffer). BNB is cheap so the buffer is generous.
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
    let buffered = gas_price.saturating_mul(120) / 100;
    Ok(buffered.saturating_mul(gas_limit as u128))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_address_lower() {
        let p = pad_address("0xAbCdEf0123456789012345678901234567890123");
        assert_eq!(p.len(), 64);
        assert_eq!(&p[24..], "abcdef0123456789012345678901234567890123");
    }

    #[test]
    fn parse_address_strips_high_bytes() {
        let w = "0000000000000000000000005c952063c7fc8610ffdb798152d69f0b9550762b";
        assert_eq!(parse_address(w), "0x5c952063c7fc8610ffdb798152d69f0b9550762b");
    }
}
