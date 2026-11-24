// src/rpc.rs — Direct eth_call via JSON-RPC (no onchainos needed for reads)
use anyhow::Context;
use serde_json::{json, Value};

/// Perform a raw `eth_call` against an RPC endpoint.
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("failed to build HTTP client")?;
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call JSON parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a 32-byte ABI-encoded address from an eth_call result (strips 0x + left-pad).
pub fn decode_address_result(hex_result: &str) -> String {
    let s = hex_result.trim_start_matches("0x");
    if s.len() >= 64 {
        format!("0x{}", &s[s.len() - 40..])
    } else {
        String::new()
    }
}

/// Call `feeLockerForToken(address token)` on the Clanker factory to resolve
/// the fee locker address for a given token.
///
/// Selector: keccak256("feeLockerForToken(address)")[0..4] = 0xb14177cb
/// Note: This may revert for tokens where the locker is stored differently;
/// callers should fall back to the config fallback address on error.
pub async fn resolve_fee_locker(
    rpc_url: &str,
    factory_addr: &str,
    token_addr: &str,
) -> anyhow::Result<String> {
    // selector: keccak256("feeLockerForToken(address)") = 0xb14177cb
    let token_padded = format!(
        "{:0>64}",
        token_addr.trim_start_matches("0x").to_lowercase()
    );
    let calldata = format!("0xb14177cb{}", token_padded);

    let result = eth_call(rpc_url, factory_addr, &calldata).await?;
    let addr = decode_address_result(&result);
    Ok(addr)
}

/// Query `tokenRewards(address token)` on a ClankerFeeLocker V4.
///
/// Selector: keccak256("tokenRewards(address)") = 0x30bd3eeb
///
/// Returns `Ok(true)` if the call succeeds and returns non-zero data (rewards exist),
/// `Ok(false)` if the call succeeds and returns empty or zero rewards,
/// or an error if the call fails.
pub async fn has_pending_rewards(
    rpc_url: &str,
    fee_locker_addr: &str,
    token_addr: &str,
) -> anyhow::Result<bool> {
    // selector: keccak256("tokenRewards(address)") = 0x30bd3eeb
    let token_padded = format!(
        "{:0>64}",
        token_addr.trim_start_matches("0x").to_lowercase()
    );
    let calldata = format!("0x30bd3eeb{}", token_padded);

    let result = eth_call(rpc_url, fee_locker_addr, &calldata).await?;
    let hex = result.trim_start_matches("0x");
    // tokenRewards returns a struct (ABI-encoded). If it returns non-empty non-zero
    // data, there is a reward config (though not necessarily claimable balance).
    // We treat any non-empty, non-all-zeros response as "rewards may exist".
    if hex.is_empty() {
        return Ok(false);
    }
    let all_zero = hex.chars().all(|c| c == '0');
    Ok(!all_zero)
}
