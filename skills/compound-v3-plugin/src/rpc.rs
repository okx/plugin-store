// src/rpc.rs — Direct eth_call queries (no onchainos required for reads)
use anyhow::Context;
use serde_json::{json, Value};

/// Low-level eth_call
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
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
        .context("RPC request failed")?
        .json()
        .await
        .context("RPC response parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("RPC error: {}", err);
    }
    Ok(resp["result"]
        .as_str()
        .unwrap_or("0x")
        .to_string())
}

/// Parse a uint256 from a 32-byte ABI-encoded hex result
pub fn parse_u128(hex_result: &str) -> anyhow::Result<u128> {
    let clean = hex_result.trim_start_matches("0x");
    if clean.len() < 64 {
        anyhow::bail!("Result too short: {}", hex_result);
    }
    let val = u128::from_str_radix(&clean[clean.len() - 32..], 16)
        .context("parse u128 failed")?;
    Ok(val)
}

/// Parse a bool from a 32-byte ABI-encoded hex result
pub fn parse_bool(hex_result: &str) -> bool {
    let clean = hex_result.trim_start_matches("0x");
    clean.ends_with('1')
}

/// Pad an address to 32 bytes (remove 0x, left-pad with zeros)
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 to 32 bytes
pub fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

// ── Comet read calls ──────────────────────────────────────────────────────────

/// Comet.getUtilization() → u128 (1e18 scaled)
pub async fn get_utilization(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x7eb71131", rpc_url).await?;
    parse_u128(&result)
}

/// Comet.getSupplyRate(uint256) → u64 (per-second, 1e18 scaled)
pub async fn get_supply_rate(comet: &str, utilization: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0xd955759d{}", pad_u128(utilization));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.getBorrowRate(uint256) → u64 (per-second, 1e18 scaled)
pub async fn get_borrow_rate(comet: &str, utilization: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x9fa83b5a{}", pad_u128(utilization));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.totalSupply() → u128
pub async fn get_total_supply(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x18160ddd", rpc_url).await?;
    parse_u128(&result)
}

/// Comet.totalBorrow() → u128
pub async fn get_total_borrow(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x8285ef40", rpc_url).await?;
    parse_u128(&result)
}

/// Comet.balanceOf(address) → u128 (supply balance of base asset)
pub async fn get_balance_of(comet: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.borrowBalanceOf(address) → u128 (borrow balance including accrued interest)
pub async fn get_borrow_balance_of(comet: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x374c49b4{}", pad_address(wallet));
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.collateralBalanceOf(address account, address asset) → u128
pub async fn get_collateral_balance_of(
    comet: &str,
    wallet: &str,
    asset: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = format!(
        "0x5c2549ee{}{}",
        pad_address(wallet),
        pad_address(asset)
    );
    let result = eth_call(comet, &data, rpc_url).await?;
    parse_u128(&result)
}

/// Comet.isBorrowCollateralized(address) → bool
pub async fn is_borrow_collateralized(comet: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<bool> {
    let data = format!("0x38aa813f{}", pad_address(wallet));
    let result = eth_call(comet, &data, rpc_url).await?;
    Ok(parse_bool(&result))
}

/// Comet.baseBorrowMin() → u128
pub async fn get_base_borrow_min(comet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(comet, "0x300e6beb", rpc_url).await?;
    parse_u128(&result)
}

/// ERC-20 balanceOf(address) → u128
pub async fn get_erc20_balance(token: &str, wallet: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(token, &data, rpc_url).await?;
    parse_u128(&result)
}

// ── CometRewards read calls ────────────────────────────────────────────────────

/// CometRewards.getRewardOwed(address comet, address account) → (token, owed)
/// Returns the owed COMP amount (u128). Returns 0 if no rewards.
pub async fn get_reward_owed(
    rewards: &str,
    comet: &str,
    wallet: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let data = format!(
        "0x41e0cad6{}{}",
        pad_address(comet),
        pad_address(wallet)
    );
    let result = eth_call(rewards, &data, rpc_url).await?;
    // Returns (address token, uint256 owed) — 2 x 32 bytes; owed is second word
    let clean = result.trim_start_matches("0x");
    if clean.len() < 128 {
        return Ok(0);
    }
    let owed_hex = &clean[64..128];
    Ok(u128::from_str_radix(owed_hex, 16).unwrap_or(0))
}

/// Simulate a Comet.withdraw(asset, amount) call from a given address.
/// Returns Ok(()) if the simulation passes; returns a descriptive error if it reverts.
/// Catches NotCollateralized() (0x14c5f7b6) and surfaces it as a clear message
/// including the market's baseBorrowMin so agents can guide users on position sizing.
pub async fn simulate_borrow(
    comet: &str,
    asset: &str,
    amount: u128,
    from: &str,
    rpc_url: &str,
    base_decimals: u8,
    base_symbol: &str,
) -> anyhow::Result<()> {
    let calldata = format!("0xf3fef3a3{}{}", pad_address(asset), pad_u128(amount));
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "from": from, "to": comet, "data": calldata }, "latest"],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("Borrow simulation RPC request failed")?
        .json()
        .await
        .context("Borrow simulation RPC parse failed")?;

    if let Some(err) = resp.get("error") {
        let data = err
            .get("data")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        if data.starts_with("0x14c5f7b6") {
            // NotCollateralized() custom error (keccak256("NotCollateralized()") = 0x14c5f7b6)
            // fetch baseBorrowMin for the actionable message
            let min_raw = get_base_borrow_min(comet, rpc_url).await.unwrap_or(0);
            let decimals_factor = 10u128.pow(base_decimals as u32) as f64;
            let min_human = min_raw as f64 / decimals_factor;
            anyhow::bail!(
                "Borrow would fail: account is not sufficiently collateralized. \
                 This market requires a minimum borrow of {min_human:.6} {base_symbol} \
                 (baseBorrowMin = {min_raw} raw units). \
                 To fix: (1) supply more collateral using 'compound-v3 supply --asset <collateral_address> --amount <amount>', \
                 then borrow at least {min_human:.6} {base_symbol}. \
                 If you already have collateral, your borrowing capacity may be too low for the requested amount — \
                 check your position with 'compound-v3 get-position'."
            );
        }
        anyhow::bail!("Borrow simulation failed: {}", err);
    }
    Ok(())
}

/// ERC-20 decimals() → u8
pub async fn get_erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    // decimals() selector: 0x313ce567
    let result = eth_call(token, "0x313ce567", rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 2 {
        return Ok(18); // safe default
    }
    let val = u8::from_str_radix(&clean[clean.len() - 2..], 16).unwrap_or(18);
    Ok(val)
}

/// Convert per-second rate (1e18 scaled) to APR percentage
pub fn rate_to_apr_pct(rate_per_sec: u128) -> f64 {
    (rate_per_sec as f64 / 1e18) * 31_536_000.0 * 100.0
}

/// Parse a human-readable decimal amount string into raw token units.
/// "0.1" with decimals=6 → 100_000
/// "1.5" with decimals=18 → 1_500_000_000_000_000_000
/// Avoids floating-point precision loss by working on the string directly.
pub fn parse_human_amount(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = amount_str.trim();
    let factor = 10u128.pow(decimals as u32);
    if let Some(dot_pos) = s.find('.') {
        let int_part: u128 = if dot_pos == 0 {
            0
        } else {
            s[..dot_pos].parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_str = &s[dot_pos + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!(
                "Amount '{}' has {} decimal places but token only supports {}",
                s, frac_str.len(), decimals
            );
        }
        let frac: u128 = if frac_str.is_empty() {
            0
        } else {
            frac_str.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_factor = 10u128.pow(decimals as u32 - frac_str.len() as u32);
        Ok(int_part * factor + frac * frac_factor)
    } else {
        let int_val: u128 = s.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        Ok(int_val * factor)
    }
}
