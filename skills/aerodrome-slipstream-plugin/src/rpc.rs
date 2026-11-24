use serde_json::{json, Value};
use crate::config::pad_address;
use tokio::time::{sleep, Duration};

/// Raw JSON-RPC eth_call with retry on rate-limit errors.
/// Retries up to 3 times with 1s / 2s / 4s backoff.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });

    let mut delay_ms = 1000u64;
    for attempt in 0..4 {
        let resp: Value = client
            .post(rpc_url)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        if let Some(err) = resp.get("error") {
            let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            // -32016 = rate limit; retry with backoff
            if code == -32016 && attempt < 3 {
                sleep(Duration::from_millis(delay_ms)).await;
                delay_ms *= 2;
                continue;
            }
            anyhow::bail!("eth_call error: {}", err);
        }
        return Ok(resp["result"].as_str().unwrap_or("0x").to_string());
    }
    anyhow::bail!("eth_call: exceeded rate limit retries")
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn strip_hex(hex: &str) -> &str {
    hex.trim_start_matches("0x")
}

fn last32(hex: &str) -> &str {
    let h = strip_hex(hex);
    if h.len() >= 64 { &h[h.len() - 64..] } else { h }
}

fn decode_u128(hex: &str) -> u128 {
    u128::from_str_radix(last32(hex), 16).unwrap_or(0)
}

fn decode_address(hex: &str) -> String {
    let h = strip_hex(hex);
    let trimmed = if h.len() >= 40 { &h[h.len() - 40..] } else { h };
    format!("0x{}", trimmed)
}

fn decode_i32(hex: &str) -> i32 {
    let h = last32(hex);
    // Take last 8 hex chars (4 bytes) and interpret as signed i32
    let last8 = if h.len() >= 8 { &h[h.len() - 8..] } else { h };
    u32::from_str_radix(last8, 16).unwrap_or(0) as i32
}

// ── ERC-20 ───────────────────────────────────────────────────────────────────

/// ERC-20 decimals() — selector 0x313ce567
pub async fn get_decimals(token: &str, rpc: &str) -> anyhow::Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc).await?;
    Ok(decode_u128(&hex) as u8)
}

/// ERC-20 allowance(owner, spender) — selector 0xdd62ed3e
pub async fn get_allowance(token: &str, owner: &str, spender: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let hex = eth_call(token, &data, rpc).await?;
    Ok(decode_u128(&hex))
}

/// Parse a human-readable decimal string into raw token units.
/// "1.5" with decimals=6 → 1_500_000
pub fn parse_human_amount(s: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = s.trim();
    let factor = 10u128.pow(decimals as u32);
    if let Some(dot) = s.find('.') {
        let int_part: u128 = if dot == 0 { 0 } else {
            s[..dot].parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_str = &s[dot + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!("Amount '{}' has {} decimal places but token supports only {}", s, frac_str.len(), decimals);
        }
        let frac: u128 = if frac_str.is_empty() { 0 } else {
            frac_str.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_factor = 10u128.pow(decimals as u32 - frac_str.len() as u32);
        Ok(int_part * factor + frac * frac_factor)
    } else {
        let v: u128 = s.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        Ok(v * factor)
    }
}

/// Format raw units back to human-readable (for display).
pub fn format_amount(raw: u128, decimals: u8) -> String {
    let factor = 10u128.pow(decimals as u32);
    let int_part = raw / factor;
    let frac_part = raw % factor;
    if frac_part == 0 {
        int_part.to_string()
    } else {
        format!("{}.{:0>width$}", int_part, frac_part, width = decimals as usize)
            .trim_end_matches('0')
            .to_string()
    }
}

// ── ERC-721 (NFT position queries) ───────────────────────────────────────────

/// ERC-721 balanceOf(address) — selector 0x70a08231 (same selector as ERC-20)
pub async fn nft_balance_of(nfpm: &str, owner: &str, rpc: &str) -> anyhow::Result<u32> {
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(nfpm, &data, rpc).await?;
    Ok(decode_u128(&hex) as u32)
}

/// ERC-721 tokenOfOwnerByIndex(address owner, uint256 index) — selector 0x2f745c59
pub async fn nft_token_of_owner_by_index(nfpm: &str, owner: &str, index: u32, rpc: &str) -> anyhow::Result<u128> {
    let data = format!(
        "0x2f745c59{}{}",
        pad_address(owner),
        format!("{:0>64x}", index)
    );
    let hex = eth_call(nfpm, &data, rpc).await?;
    Ok(decode_u128(&hex))
}

// ── CLFactory ────────────────────────────────────────────────────────────────

/// CLFactory.getPool(address tokenA, address tokenB, int24 tickSpacing) → address
/// Selector: keccak("getPool(address,address,int24)") = 0x28af8d0b
pub async fn cl_get_pool(factory: &str, token_a: &str, token_b: &str, tick_spacing: i32, rpc: &str) -> anyhow::Result<String> {
    let ta = pad_address(token_a);
    let tb = pad_address(token_b);
    // int24 tickSpacing encoded as int256 (padded 32 bytes, two's complement)
    // Encode int24 tickSpacing as ABI int256 (32 bytes, two's complement via sign-extend to i64→u64)
    let ts = format!("{:0>64x}", tick_spacing as i64 as u64);
    let data = format!("0x28af8d0b{}{}{}", ta, tb, ts);
    let hex = eth_call(factory, &data, rpc).await?;
    Ok(decode_address(&hex))
}

// ── Pool slot0 ───────────────────────────────────────────────────────────────

/// Pool.slot0() — returns (sqrtPriceX96, tick, ...).
/// Selector: 0x3850c7bd
/// Returns (sqrtPriceX96: u128, tick: i32)
pub async fn pool_slot0(pool: &str, rpc: &str) -> anyhow::Result<(u128, i32)> {
    let hex = eth_call(pool, "0x3850c7bd", rpc).await?;
    let clean = strip_hex(&hex);
    if clean.len() < 128 {
        anyhow::bail!("slot0 returned insufficient data");
    }
    // ABI layout: each slot is 64 hex chars (32 bytes), right-aligned.
    // Slot 0 (chars  0:64): sqrtPriceX96 uint160 — lower 16 bytes are sufficient for all practical prices
    // Slot 1 (chars 64:128): tick int24 — sign-extended to 256 bits; take last 8 chars as int32
    let sqrt_price = u128::from_str_radix(&clean[32..64], 16).unwrap_or(0);
    let tick_last8 = &clean[120..128];
    let tick = u32::from_str_radix(tick_last8, 16).unwrap_or(0) as i32;
    Ok((sqrt_price, tick))
}

/// Pool.tickSpacing() — selector 0xd0c93a7c
pub async fn pool_tick_spacing(pool: &str, rpc: &str) -> anyhow::Result<i32> {
    let hex = eth_call(pool, "0xd0c93a7c", rpc).await?;
    Ok(decode_i32(&hex))
}

/// Pool.fee() — selector 0xddca3f43
pub async fn pool_fee(pool: &str, rpc: &str) -> anyhow::Result<u32> {
    let hex = eth_call(pool, "0xddca3f43", rpc).await?;
    Ok(decode_u128(&hex) as u32)
}

/// Pool.liquidity() — selector 0x1a686502
pub async fn pool_liquidity(pool: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(pool, "0x1a686502", rpc).await?;
    Ok(decode_u128(&hex))
}

/// Pool.token0() — selector 0x0dfe1681
pub async fn pool_token0(pool: &str, rpc: &str) -> anyhow::Result<String> {
    let hex = eth_call(pool, "0x0dfe1681", rpc).await?;
    Ok(decode_address(&hex))
}

// ── Compute price from sqrtPriceX96 ─────────────────────────────────────────

/// Convert sqrtPriceX96 to human-readable price of token1 per token0.
/// price = (sqrtPriceX96 / 2^96)^2  * (10^decimals0 / 10^decimals1)
pub fn sqrt_price_to_human(sqrt_price_x96: u128, decimals0: u8, decimals1: u8) -> f64 {
    if sqrt_price_x96 == 0 { return 0.0; }
    // Use f64 — safe for display purposes (not for tx math)
    let sp = sqrt_price_x96 as f64;
    let q96 = 2f64.powi(96);
    let price_raw = (sp / q96).powi(2);
    let decimal_adj = 10f64.powi(decimals0 as i32 - decimals1 as i32);
    price_raw * decimal_adj
}

// ── NFPM positions ───────────────────────────────────────────────────────────

/// NFPM.positions(uint256 tokenId) — selector 0x99fbab88
/// Returns a struct with token0, token1, tickSpacing, tickLower, tickUpper, liquidity, tokensOwed0, tokensOwed1
pub struct PositionInfo {
    pub token0: String,
    pub token1: String,
    pub tick_spacing: i32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub tokens_owed0: u128,
    pub tokens_owed1: u128,
}

pub async fn nfpm_positions(nfpm: &str, token_id: u128, rpc: &str) -> anyhow::Result<PositionInfo> {
    let data = format!("0x99fbab88{}", format!("{:0>64x}", token_id));
    let hex = eth_call(nfpm, &data, rpc).await?;
    let clean = strip_hex(&hex);
    if clean.len() < 64 * 12 {
        anyhow::bail!("positions({}) returned insufficient data ({} chars)", token_id, clean.len());
    }
    // ABI decoding: each field is 32 bytes (64 hex chars)
    // Fields in order (from Slipstream NFPM):
    // 0: nonce (uint96)
    // 1: operator (address)
    // 2: token0 (address)
    // 3: token1 (address)
    // 4: tickSpacing (int24)
    // 5: tickLower (int24)
    // 6: tickUpper (int24)
    // 7: liquidity (uint128)
    // 8: feeGrowthInside0LastX128 (uint256)
    // 9: feeGrowthInside1LastX128 (uint256)
    // 10: tokensOwed0 (uint128)
    // 11: tokensOwed1 (uint128)
    let field = |i: usize| &clean[i * 64..(i + 1) * 64];
    Ok(PositionInfo {
        token0: format!("0x{}", &field(2)[24..]),
        token1: format!("0x{}", &field(3)[24..]),
        tick_spacing: u32::from_str_radix(&field(4)[56..], 16).unwrap_or(0) as i32,
        tick_lower:   u32::from_str_radix(&field(5)[56..], 16).unwrap_or(0) as i32,
        tick_upper:   u32::from_str_radix(&field(6)[56..], 16).unwrap_or(0) as i32,
        liquidity:    u128::from_str_radix(field(7), 16).unwrap_or(0),
        tokens_owed0: u128::from_str_radix(field(10), 16).unwrap_or(0),
        tokens_owed1: u128::from_str_radix(field(11), 16).unwrap_or(0),
    })
}

