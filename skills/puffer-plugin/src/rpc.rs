use anyhow::Context;
use serde_json::{json, Value};

use crate::config::pad_address;

/// Perform an eth_call via JSON-RPC.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            {"to": to, "data": data},
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
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode last 32 bytes of hex response as u128 (returns the error if the response is not a clean hex word).
/// Use this instead of `unwrap_or(0)` so RPC/decode failures do not silently return zero (→ EVM-012).
fn decode_u128(hex: &str, label: &str) -> anyhow::Result<u128> {
    let clean = hex.trim_start_matches("0x");
    if clean.is_empty() || clean == "0x" {
        anyhow::bail!("{} returned empty data", label);
    }
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    u128::from_str_radix(trimmed, 16)
        .map_err(|e| anyhow::anyhow!("{} decode failed ({}): raw={}", label, e, hex))
}

/// ERC-20 balanceOf(address) -> uint256. Selector: 0x70a08231
pub async fn get_balance(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(token, &data, rpc_url).await?;
    decode_u128(&hex, "balanceOf")
}

// Native ETH balance, gas price, and gas estimation now go through onchainos (onchainos.rs).
// Kept only contract-level view functions here since onchainos doesn't expose generic eth_call.

/// PufferWithdrawalManager.getMaxWithdrawalAmount() -> uint256. Selector: 0x9ce7f670
/// Upper bound on a single 2-step withdrawal request (governance-tunable).
pub async fn get_max_withdrawal_amount(manager: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(manager, "0x9ce7f670", rpc_url).await?;
    decode_u128(&hex, "getMaxWithdrawalAmount")
}

/// ERC-20 allowance(owner, spender) -> uint256. Selector: 0xdd62ed3e
pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = format!(
        "0xdd62ed3e{}{}",
        pad_address(owner),
        pad_address(spender),
    );
    let hex = eth_call(token, &data, rpc_url).await?;
    decode_u128(&hex, "allowance")
}

// ============================================================
// PufferVaultV5 read-only
// ============================================================

/// convertToAssets(uint256 shares) -> uint256. Selector: 0x07a2d13a
/// Returns ETH value of given pufETH share amount (ignores fee).
pub async fn convert_to_assets(vault: &str, shares: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x07a2d13a{:0>64x}", shares);
    let hex = eth_call(vault, &data, rpc_url).await?;
    decode_u128(&hex, "convertToAssets")
}

/// previewRedeem(uint256 shares) -> uint256. Selector: 0x4cdad506
/// Returns the WETH amount user would receive for redeeming `shares` pufETH
/// **after** subtracting the exit fee.
pub async fn preview_redeem(vault: &str, shares: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x4cdad506{:0>64x}", shares);
    let hex = eth_call(vault, &data, rpc_url).await?;
    decode_u128(&hex, "previewRedeem")
}

/// totalAssets() -> uint256. Selector: 0x01e1d114
pub async fn total_assets(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(vault, "0x01e1d114", rpc_url).await?;
    decode_u128(&hex, "totalAssets")
}

/// getTotalExitFeeBasisPoints() -> uint256. Selector: 0x0116c1fa
/// Basis points applied on 1-step instant withdraw (e.g. 100 = 1%).
pub async fn get_total_exit_fee_bps(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(vault, "0x0116c1fa", rpc_url).await?;
    decode_u128(&hex, "getTotalExitFeeBasisPoints")
}

/// maxRedeem(address owner) -> uint256. Selector: 0xd905777e
/// Maximum pufETH shares the owner can redeem right now (subject to liquidity).
pub async fn max_redeem(vault: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0xd905777e{}", pad_address(owner));
    let hex = eth_call(vault, &data, rpc_url).await?;
    decode_u128(&hex, "maxRedeem")
}

// ============================================================
// PufferWithdrawalManager read-only
// ============================================================

/// getFinalizedWithdrawalBatch() -> uint256. Selector: 0x90294b42
/// Latest finalized batch index (inclusive). Batches with index ≤ this value can be claimed.
pub async fn get_finalized_batch(manager: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(manager, "0x90294b42", rpc_url).await?;
    decode_u128(&hex, "getFinalizedWithdrawalBatch")
}

/// getWithdrawalsLength() -> uint256. Selector: 0x2d9b7f4d
pub async fn get_withdrawals_length(manager: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(manager, "0x2d9b7f4d", rpc_url).await?;
    decode_u128(&hex, "getWithdrawalsLength")
}

/// getWithdrawal(uint256 idx) -> (uint128 pufETHAmount, uint128 pufETHToETHExchangeRate, address recipient)
/// Selector: 0x8a4fb16a
///
/// Returns (pufeth_amount, rate_e18, recipient_lower_hex) or None if the struct is empty (zero fields).
pub async fn get_withdrawal(
    manager: &str,
    idx: u128,
    rpc_url: &str,
) -> anyhow::Result<Option<(u128, u128, String)>> {
    let data = format!("0x8a4fb16a{:0>64x}", idx);
    let hex = eth_call(manager, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 192 {
        return Ok(None);
    }
    let puf_hex = &clean[0..64];
    let rate_hex = &clean[64..128];
    let recipient_hex = &clean[128..192];
    let puf = u128::from_str_radix(puf_hex, 16)
        .map_err(|e| anyhow::anyhow!("getWithdrawal.pufETHAmount decode failed: {}", e))?;
    let rate = u128::from_str_radix(rate_hex, 16)
        .map_err(|e| anyhow::anyhow!("getWithdrawal.rate decode failed: {}", e))?;
    let recipient = format!("0x{}", &recipient_hex[24..]); // last 20 bytes
    if puf == 0 && rate == 0 && recipient == "0x0000000000000000000000000000000000000000" {
        return Ok(None);
    }
    Ok(Some((puf, rate, recipient)))
}
