use serde_json::{json, Value};
use tokio::time::{sleep, Duration};
use crate::config::{pad_address, pad_bool, pad_u256};

/// Raw JSON-RPC eth_call with retry on rate-limit errors.
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

// ── ERC-20 ───────────────────────────────────────────────────────────────────

/// ERC-20 decimals() — selector 0x313ce567
pub async fn get_decimals(token: &str, rpc: &str) -> anyhow::Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc).await?;
    Ok(decode_u128(&hex) as u8)
}

/// ERC-20 balanceOf(address) — selector 0x70a08231
pub async fn get_balance_of(token: &str, owner: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(token, &data, rpc).await?;
    Ok(decode_u128(&hex))
}

/// ERC-20 allowance(owner, spender) — selector 0xdd62ed3e
pub async fn get_allowance(token: &str, owner: &str, spender: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let hex = eth_call(token, &data, rpc).await?;
    Ok(decode_u128(&hex))
}

/// Poll `allowance(owner, spender)` until it reaches at least `min_amount`,
/// or the timeout elapses. Used right after submitting an ERC-20 approve tx
/// to make sure the next contract call (swap / addLiquidity / removeLiquidity)
/// sees the updated allowance — relying on a fixed `sleep` would
/// occasionally race the approve receipt and surface
/// `ERC20: insufficient allowance` from the downstream call.
///
/// Polls every ~1.5s. 60s total wall-clock budget is generous on Base
/// (typical block time 2s, soft finality 1 confirmation), and bails loudly
/// rather than silently broadcasting a doomed follow-up.
pub async fn wait_for_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    min_amount: u128,
    rpc: &str,
) -> anyhow::Result<()> {
    const TIMEOUT_SECS: u64 = 60;
    const POLL_INTERVAL_MS: u64 = 1_500;

    let start = std::time::Instant::now();
    loop {
        let allowance = get_allowance(token, owner, spender, rpc).await.unwrap_or(0);
        if allowance >= min_amount {
            return Ok(());
        }
        if start.elapsed().as_secs() >= TIMEOUT_SECS {
            anyhow::bail!(
                "allowance still {} < {} after {}s — approve tx likely failed or RPC is lagging",
                allowance, min_amount, TIMEOUT_SECS
            );
        }
        sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

/// ERC-20 totalSupply() — selector 0x18160ddd
pub async fn get_total_supply(token: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(token, "0x18160ddd", rpc).await?;
    Ok(decode_u128(&hex))
}

/// Parse a human-readable decimal string into raw token units.
pub fn parse_human_amount(s: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = s.trim();
    let factor = 10u128.pow(decimals as u32);
    if let Some(dot) = s.find('.') {
        let int_part: u128 = if dot == 0 { 0 } else {
            s[..dot].parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_str = &s[dot + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!(
                "Amount '{}' has {} decimal places but token only supports {}",
                s, frac_str.len(), decimals
            );
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
    if decimals == 0 { return raw.to_string(); }
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

// ── AMM Factory ───────────────────────────────────────────────────────────────

/// Factory.getPool(tokenA, tokenB, stable) → address
/// Selector: 0x79bc57d5 (verified on-chain)
pub async fn amm_get_pool(
    factory: &str,
    token_a: &str,
    token_b: &str,
    stable: bool,
    rpc: &str,
) -> anyhow::Result<String> {
    let data = format!(
        "0x79bc57d5{}{}{}",
        pad_address(token_a),
        pad_address(token_b),
        pad_bool(stable)
    );
    let hex = eth_call(factory, &data, rpc).await?;
    Ok(decode_address(&hex))
}

// ── AMM Pool ──────────────────────────────────────────────────────────────────

/// Pool.stable() → bool
/// Selector: 0x22be3de1 (verified on-chain)
pub async fn pool_is_stable(pool: &str, rpc: &str) -> anyhow::Result<bool> {
    let hex = eth_call(pool, "0x22be3de1", rpc).await?;
    Ok(decode_u128(&hex) != 0)
}

/// Pool.getReserves() → (uint256 reserve0, uint256 reserve1, uint256 blockTimestampLast)
/// Selector: 0x0902f1ac (verified on-chain)
pub async fn pool_get_reserves(pool: &str, rpc: &str) -> anyhow::Result<(u128, u128)> {
    let hex = eth_call(pool, "0x0902f1ac", rpc).await?;
    let clean = strip_hex(&hex);
    if clean.len() < 128 {
        anyhow::bail!("getReserves returned insufficient data");
    }
    let r0 = u128::from_str_radix(&clean[0..64], 16).unwrap_or(0);
    let r1 = u128::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    Ok((r0, r1))
}

/// Pool.token0() — selector 0x0dfe1681
pub async fn pool_token0(pool: &str, rpc: &str) -> anyhow::Result<String> {
    let hex = eth_call(pool, "0x0dfe1681", rpc).await?;
    Ok(decode_address(&hex))
}

/// Pool.token1() — selector 0xd21220a7
pub async fn pool_token1(pool: &str, rpc: &str) -> anyhow::Result<String> {
    let hex = eth_call(pool, "0xd21220a7", rpc).await?;
    Ok(decode_address(&hex))
}

// ── AMM Router view calls ─────────────────────────────────────────────────────

/// Router.getAmountsOut(amountIn, Route[1]) → uint256[] amounts
/// Selector: 0x5509a1ac (verified via eth_utils keccak256)
///
/// ABI encoding for Route[] with 1 element:
///   [4] selector
///   [32] amountIn
///   [32] offset = 0x40 (64)
///   [32] routes.length = 1
///   [32] routes[0].from
///   [32] routes[0].to
///   [32] routes[0].stable
///   [32] routes[0].factory
///
/// Returns uint256[]: [amountIn, amountOut]
pub async fn router_get_amounts_out(
    router: &str,
    factory: &str,
    amount_in: u128,
    token_from: &str,
    token_to: &str,
    stable: bool,
    rpc: &str,
) -> anyhow::Result<u128> {
    let data = format!(
        "0x5509a1ac{}{}{}{}{}{}",
        pad_u256(amount_in),
        "0000000000000000000000000000000000000000000000000000000000000040", // offset
        "0000000000000000000000000000000000000000000000000000000000000001", // length=1
        pad_address(token_from),
        pad_address(token_to),
        pad_bool(stable),
    ) + &pad_address(factory);
    let hex = eth_call(router, &data, rpc).await?;
    let clean = strip_hex(&hex);
    // Result is uint256[]: offset(64) + length(64) + amounts[0](64) + amounts[1](64)
    // amountOut is at position 3 * 64 = 192
    if clean.len() < 256 {
        anyhow::bail!("getAmountsOut returned insufficient data");
    }
    let amount_out = u128::from_str_radix(&clean[192..256], 16).unwrap_or(0);
    Ok(amount_out)
}

/// Router.quoteAddLiquidity(tokenA, tokenB, stable, factory, amountADesired, amountBDesired)
/// → (uint256 amountA, uint256 amountB, uint256 liquidity)
/// Selector: 0xce700c29
pub async fn router_quote_add_liquidity(
    router: &str,
    factory: &str,
    token_a: &str,
    token_b: &str,
    stable: bool,
    amount_a: u128,
    amount_b: u128,
    rpc: &str,
) -> anyhow::Result<(u128, u128, u128)> {
    let data = format!(
        "0xce700c29{}{}{}{}{}{}",
        pad_address(token_a),
        pad_address(token_b),
        pad_bool(stable),
        pad_address(factory),
        pad_u256(amount_a),
        pad_u256(amount_b),
    );
    let hex = eth_call(router, &data, rpc).await?;
    let clean = strip_hex(&hex);
    if clean.len() < 192 {
        anyhow::bail!("quoteAddLiquidity returned insufficient data");
    }
    let used_a   = u128::from_str_radix(&clean[0..64],   16).unwrap_or(0);
    let used_b   = u128::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    let lp_out   = u128::from_str_radix(&clean[128..192],16).unwrap_or(0);
    Ok((used_a, used_b, lp_out))
}
