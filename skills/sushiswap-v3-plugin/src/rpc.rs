use serde_json::{json, Value};
use crate::config::pad_address;
use tokio::time::{sleep, Duration};

pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0", "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"], "id": 1
    });
    let mut delay_ms = 1000u64;
    for attempt in 0..4 {
        let resp: Value = client.post(rpc_url).json(&body).send().await?.json().await?;
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

fn strip_hex(hex: &str) -> &str { hex.trim_start_matches("0x") }
fn last32(hex: &str) -> &str {
    let h = strip_hex(hex);
    if h.len() >= 64 { &h[h.len() - 64..] } else { h }
}
fn decode_u128(hex: &str) -> u128 { u128::from_str_radix(last32(hex), 16).unwrap_or(0) }
fn decode_address(hex: &str) -> String {
    let h = strip_hex(hex);
    let trimmed = if h.len() >= 40 { &h[h.len() - 40..] } else { h };
    format!("0x{}", trimmed)
}

pub async fn get_decimals(token: &str, rpc: &str) -> anyhow::Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc).await?;
    Ok(decode_u128(&hex) as u8)
}

pub async fn get_allowance(token: &str, owner: &str, spender: &str, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let hex = eth_call(token, &data, rpc).await?;
    Ok(decode_u128(&hex))
}

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

pub fn format_amount(raw: u128, decimals: u8) -> String {
    let factor = 10u128.pow(decimals as u32);
    let int_part = raw / factor;
    let frac_part = raw % factor;
    if frac_part == 0 { int_part.to_string() } else {
        format!("{}.{:0>width$}", int_part, frac_part, width = decimals as usize)
            .trim_end_matches('0').to_string()
    }
}

pub async fn nft_balance_of(nfpm: &str, owner: &str, rpc: &str) -> anyhow::Result<u32> {
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(nfpm, &data, rpc).await?;
    Ok(decode_u128(&hex) as u32)
}

pub async fn nft_token_of_owner_by_index(nfpm: &str, owner: &str, index: u32, rpc: &str) -> anyhow::Result<u128> {
    let data = format!("0x2f745c59{}{}", pad_address(owner), format!("{:0>64x}", index));
    let hex = eth_call(nfpm, &data, rpc).await?;
    Ok(decode_u128(&hex))
}

/// UniV3Factory.getPool(address,address,uint24) → address
/// Selector: 0x1698ee82
pub async fn v3_get_pool(factory: &str, token_a: &str, token_b: &str, fee: u32, rpc: &str) -> anyhow::Result<String> {
    let ta = pad_address(token_a);
    let tb = pad_address(token_b);
    let fee_enc = format!("{:0>64x}", fee);
    let data = format!("0x1698ee82{}{}{}", ta, tb, fee_enc);
    let hex = eth_call(factory, &data, rpc).await?;
    Ok(decode_address(&hex))
}

pub async fn pool_slot0(pool: &str, rpc: &str) -> anyhow::Result<(u128, i32)> {
    let hex = eth_call(pool, "0x3850c7bd", rpc).await?;
    let clean = strip_hex(&hex);
    if clean.len() < 128 { anyhow::bail!("slot0 returned insufficient data"); }
    let sqrt_price = u128::from_str_radix(&clean[32..64], 16).unwrap_or(0);
    let tick_last8 = &clean[120..128];
    let tick = u32::from_str_radix(tick_last8, 16).unwrap_or(0) as i32;
    Ok((sqrt_price, tick))
}

pub async fn pool_fee(pool: &str, rpc: &str) -> anyhow::Result<u32> {
    let hex = eth_call(pool, "0xddca3f43", rpc).await?;
    Ok(decode_u128(&hex) as u32)
}

pub async fn pool_liquidity(pool: &str, rpc: &str) -> anyhow::Result<u128> {
    let hex = eth_call(pool, "0x1a686502", rpc).await?;
    Ok(decode_u128(&hex))
}

pub async fn pool_token0(pool: &str, rpc: &str) -> anyhow::Result<String> {
    let hex = eth_call(pool, "0x0dfe1681", rpc).await?;
    Ok(decode_address(&hex))
}

pub fn sqrt_price_to_human(sqrt_price_x96: u128, decimals0: u8, decimals1: u8) -> f64 {
    if sqrt_price_x96 == 0 { return 0.0; }
    let sp = sqrt_price_x96 as f64;
    let q96 = 2f64.powi(96);
    let price_raw = (sp / q96).powi(2);
    let decimal_adj = 10f64.powi(decimals0 as i32 - decimals1 as i32);
    price_raw * decimal_adj
}

/// UniV3 NFPM.positions(uint256 tokenId) — selector 0x99fbab88
/// Returns struct with fee (uint24) at field[4] (differs from Slipstream which has tickSpacing)
pub struct PositionInfo {
    pub token0: String,
    pub token1: String,
    pub fee: u32,
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
    // UniV3 NFPM positions ABI layout (each field = 32 bytes = 64 hex chars):
    // 0: nonce, 1: operator, 2: token0, 3: token1, 4: fee (uint24),
    // 5: tickLower, 6: tickUpper, 7: liquidity, 8: feeGrowth0, 9: feeGrowth1,
    // 10: tokensOwed0, 11: tokensOwed1
    let field = |i: usize| &clean[i * 64..(i + 1) * 64];
    Ok(PositionInfo {
        token0:       format!("0x{}", &field(2)[24..]),
        token1:       format!("0x{}", &field(3)[24..]),
        fee:          u32::from_str_radix(&field(4)[56..], 16).unwrap_or(0),
        tick_lower:   u32::from_str_radix(&field(5)[56..], 16).unwrap_or(0) as i32,
        tick_upper:   u32::from_str_radix(&field(6)[56..], 16).unwrap_or(0) as i32,
        liquidity:    u128::from_str_radix(field(7), 16).unwrap_or(0),
        tokens_owed0: u128::from_str_radix(field(10), 16).unwrap_or(0),
        tokens_owed1: u128::from_str_radix(field(11), 16).unwrap_or(0),
    })
}

/// Call Sushi Swap API and return (amount_out_raw, router_to, calldata).
pub async fn sushi_quote(
    chain_id: u64,
    token_in: &str,
    token_out: &str,
    amount_in_raw: u128,
    slippage: f64,
    sender: &str,
) -> anyhow::Result<(u128, String, String)> {
    let url = format!(
        "https://api.sushi.com/swap/v7/{}?tokenIn={}&tokenOut={}&amount={}&maxSlippage={}&sender={}&includeTransaction=true",
        chain_id, token_in, token_out, amount_in_raw,
        slippage / 100.0,
        sender
    );
    let client = reqwest::Client::new();
    let resp: Value = client
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await?
        .json()
        .await?;

    let status = resp["status"].as_str().unwrap_or("Unknown");
    if status != "Success" {
        anyhow::bail!("Sushi API returned status '{}'. No route found for this token pair on chain {}.", status, chain_id);
    }
    let amount_out: u128 = resp["assumedAmountOut"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| resp["assumedAmountOut"].as_u64().map(|v| v as u128))
        .ok_or_else(|| anyhow::anyhow!("Sushi API: missing assumedAmountOut"))?;
    let to = resp["tx"]["to"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Sushi API: missing tx.to"))?
        .to_string();
    let data = resp["tx"]["data"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Sushi API: missing tx.data"))?
        .to_string();
    Ok((amount_out, to, data))
}
