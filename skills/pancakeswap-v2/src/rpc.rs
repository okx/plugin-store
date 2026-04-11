// src/rpc.rs — Direct eth_call helpers (no onchainos needed)
use anyhow::Context;
use serde_json::{json, Value};

/// Low-level eth_call. Returns the hex-encoded result string.
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
    let result = resp["result"]
        .as_str()
        .unwrap_or("0x")
        .to_string();
    Ok(result)
}

/// eth_blockNumber — get current block timestamp (unix seconds) via eth_getBlockByNumber
pub async fn current_timestamp(rpc_url: &str) -> anyhow::Result<u64> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBlockByNumber",
        "params": ["latest", false],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    let ts_hex = resp["result"]["timestamp"].as_str().unwrap_or("0x0");
    Ok(u64::from_str_radix(ts_hex.trim_start_matches("0x"), 16).unwrap_or(0))
}

/// Decode a 32-byte ABI uint256 hex string to u128
pub fn decode_u128(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    // Take last 32 hex chars (16 bytes = u128)
    let trimmed = if clean.len() > 32 {
        &clean[clean.len() - 32..]
    } else {
        clean
    };
    u128::from_str_radix(trimmed, 16).unwrap_or(0)
}

/// Decode a 32-byte ABI address (last 20 bytes = 40 hex chars)
pub fn decode_address(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        format!("0x{:0>40}", clean)
    }
}

// ─── Pair / Factory helpers ────────────────────────────────────────────────

/// Factory.getPair(tokenA, tokenB) → pair address
/// Selector: 0xe6a43905
pub async fn factory_get_pair(
    factory: &str,
    token_a: &str,
    token_b: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let a = format!("{:0>64}", token_a.trim_start_matches("0x").trim_start_matches("0X"));
    let b = format!("{:0>64}", token_b.trim_start_matches("0x").trim_start_matches("0X"));
    let data = format!("0xe6a43905{}{}", a, b);
    let result = eth_call(factory, &data, rpc_url).await?;
    Ok(decode_address(&result))
}

/// Pair.getReserves() → (reserve0, reserve1, blockTimestampLast)
/// Selector: 0x0902f1ac
/// Returns: (reserve0_u128, reserve1_u128, timestamp_u32)
pub async fn pair_get_reserves(pair: &str, rpc_url: &str) -> anyhow::Result<(u128, u128, u32)> {
    let data = "0x0902f1ac";
    let result = eth_call(pair, data, rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 192 {
        anyhow::bail!("getReserves returned short data");
    }
    let r0 = decode_u128(&clean[0..64]);
    let r1 = decode_u128(&clean[64..128]);
    let ts_hex = &clean[128..192];
    let ts = u32::from_str_radix(&ts_hex[ts_hex.len().saturating_sub(8)..], 16).unwrap_or(0);
    Ok((r0, r1, ts))
}

/// Pair.token0() → address
/// Selector: 0x0dfe1681
pub async fn pair_token0(pair: &str, rpc_url: &str) -> anyhow::Result<String> {
    let result = eth_call(pair, "0x0dfe1681", rpc_url).await?;
    Ok(decode_address(&result))
}

/// ERC-20 balanceOf(address) → u128
/// Selector: 0x70a08231
pub async fn erc20_balance_of(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x").trim_start_matches("0X"));
    let data = format!("0x70a08231{}", owner_padded);
    let result = eth_call(token, &data, rpc_url).await?;
    Ok(decode_u128(&result))
}

/// ERC-20 totalSupply() → u128
/// Selector: 0x18160ddd
pub async fn erc20_total_supply(token: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let result = eth_call(token, "0x18160ddd", rpc_url).await?;
    Ok(decode_u128(&result))
}

/// ERC-20 allowance(owner, spender) → u128
/// Selector: 0xdd62ed3e
pub async fn erc20_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let o = format!("{:0>64}", owner.trim_start_matches("0x").trim_start_matches("0X"));
    let s = format!("{:0>64}", spender.trim_start_matches("0x").trim_start_matches("0X"));
    let data = format!("0xdd62ed3e{}{}", o, s);
    let result = eth_call(token, &data, rpc_url).await?;
    Ok(decode_u128(&result))
}

/// Router02.getAmountsOut(amountIn, path[]) → amounts[]
/// Selector: 0xd06ca61f
/// Returns the full amounts array (as u128 vec)
pub async fn router_get_amounts_out(
    router: &str,
    amount_in: u128,
    path: &[&str],
    rpc_url: &str,
) -> anyhow::Result<Vec<u128>> {
    // ABI encode: (uint256 amountIn, address[] path)
    // Layout:
    //   [0..32]   amountIn
    //   [32..64]  offset to path array = 64 (0x40)
    //   [64..96]  path.len
    //   [96..]    path addresses (each 32 bytes)
    let amount_hex = format!("{:064x}", amount_in);
    let offset = "0000000000000000000000000000000000000000000000000000000000000040";
    let path_len = format!("{:064x}", path.len());
    let mut path_bytes = String::new();
    for addr in path {
        path_bytes.push_str(&format!(
            "{:0>64}",
            addr.trim_start_matches("0x").trim_start_matches("0X")
        ));
    }
    let calldata = format!("0xd06ca61f{}{}{}{}", amount_hex, offset, path_len, path_bytes);
    let result = eth_call(router, &calldata, rpc_url).await?;
    parse_amounts_out(&result, path.len())
}

/// Parse getAmountsOut return value: (uint256[] amounts)
/// ABI encoding: offset(32) + length(32) + values...
fn parse_amounts_out(hex: &str, path_len: usize) -> anyhow::Result<Vec<u128>> {
    let clean = hex.trim_start_matches("0x");
    // offset word (0x20) + array length + values
    let min_len = (2 + path_len) * 64;
    if clean.len() < min_len {
        anyhow::bail!("getAmountsOut returned too-short data");
    }
    // offset is at [0..64], array length at [64..128], values at [128..]
    let arr_len = usize::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    let mut amounts = Vec::with_capacity(arr_len);
    for i in 0..arr_len {
        let start = 128 + i * 64;
        let end = start + 64;
        if end > clean.len() {
            break;
        }
        amounts.push(decode_u128(&clean[start..end]));
    }
    Ok(amounts)
}

/// ERC-20 decimals() → u8
/// Selector: 0x313ce567
pub async fn erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let result = eth_call(token, "0x313ce567", rpc_url).await?;
    let v = decode_u128(&result);
    Ok(v as u8)
}
/// ERC-20 decimals() → u8 via raw hex decode. Falls back to 18 on error.
pub async fn get_erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let result = eth_call(token, "0x313ce567", rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 2 { return Ok(18); }
    Ok(u8::from_str_radix(&clean[clean.len() - 2..], 16).unwrap_or(18))
}

/// Parse a human-readable decimal amount string into raw token units.
pub fn parse_human_amount(amount_str: &str, decimals: u8) -> anyhow::Result<u128> {
    let s = amount_str.trim();
    let factor = 10u128.pow(decimals as u32);
    if let Some(dot_pos) = s.find('.') {
        let int_part: u128 = if dot_pos == 0 { 0 } else {
            s[..dot_pos].parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_str = &s[dot_pos + 1..];
        if frac_str.len() > decimals as usize {
            anyhow::bail!("Amount '{}' has {} decimal places but token only supports {}", s, frac_str.len(), decimals);
        }
        let frac: u128 = if frac_str.is_empty() { 0 } else {
            frac_str.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?
        };
        let frac_factor = 10u128.pow(decimals as u32 - frac_str.len() as u32);
        Ok(int_part * factor + frac * frac_factor)
    } else {
        let int_val: u128 = s.parse().map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", s))?;
        Ok(int_val * factor)
    }
}

/// ERC-20 symbol() → String (ABI-encoded)
/// Selector: 0x95d89b41
pub async fn erc20_symbol(token: &str, rpc_url: &str) -> anyhow::Result<String> {
    let result = eth_call(token, "0x95d89b41", rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    // ABI string: offset(32) + length(32) + data
    if clean.len() < 128 {
        return Ok("?".to_string());
    }
    let str_len = usize::from_str_radix(&clean[64..128], 16).unwrap_or(0);
    let data_hex = &clean[128..128 + str_len * 2];
    let bytes = hex::decode(data_hex).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Validate Router02 by calling factory() and WETH() and checking they match expected values.
pub async fn validate_router(
    router: &str,
    expected_factory: &str,
    expected_weth: &str,
    rpc_url: &str,
) -> anyhow::Result<()> {
    // factory() selector: 0xc45a0155
    let factory_result = eth_call(router, "0xc45a0155", rpc_url).await?;
    let actual_factory = decode_address(&factory_result);
    if actual_factory.to_lowercase() != expected_factory.to_lowercase() {
        anyhow::bail!(
            "Router02 factory() mismatch: got {}, expected {}",
            actual_factory,
            expected_factory
        );
    }
    // WETH() selector: 0xad5c4648
    let weth_result = eth_call(router, "0xad5c4648", rpc_url).await?;
    let actual_weth = decode_address(&weth_result);
    if actual_weth.to_lowercase() != expected_weth.to_lowercase() {
        anyhow::bail!(
            "Router02 WETH() mismatch: got {}, expected {}",
            actual_weth,
            expected_weth
        );
    }
    Ok(())
}
