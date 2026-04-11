use anyhow::Context;
use serde_json::{json, Value};

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

/// Check ERC-20 allowance.
/// allowance(address owner, address spender) -> uint256
/// Selector: 0xdd62ed3e
pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x"));
    let data = format!("0xdd62ed3e{}{}", owner_padded, spender_padded);
    let hex = eth_call(token, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// Get ERC-20 balance.
/// balanceOf(address) -> uint256
/// Selector: 0x70a08231
pub async fn get_balance(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let data = format!("0x70a08231{}", owner_padded);
    let hex = eth_call(token, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// PoolFactory.getPool(address tokenA, address tokenB, bool stable) -> address
/// Selector: 0x79bc57d5
pub async fn factory_get_pool(
    token_a: &str,
    token_b: &str,
    stable: bool,
    factory: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let ta = format!("{:0>64}", token_a.trim_start_matches("0x"));
    let tb = format!("{:0>64}", token_b.trim_start_matches("0x"));
    let s = format!("{:0>64x}", stable as u64);
    let data = format!("0x79bc57d5{}{}{}", ta, tb, s);
    let hex = eth_call(factory, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let addr = if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    };
    Ok(addr)
}

/// Router.getAmountsOut(uint256 amountIn, Route[] routes) -> uint256[]
/// Selector: 0x5509a1ac
/// For single hop: routes = [{from, to, stable, factory}]
/// Returns array of amounts: [amountIn, amountOut]
pub async fn router_get_amounts_out(
    router: &str,
    amount_in: u128,
    token_in: &str,
    token_out: &str,
    stable: bool,
    factory: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let amount_in_hex = format!("{:0>64x}", amount_in);
    // offset to routes array (2 static words: amountIn + offset = 2x32 = 64 = 0x40)
    let routes_offset = format!("{:0>64x}", 0x40u64);
    let routes_length = format!("{:0>64x}", 1u64);
    let route_from = format!("{:0>64}", token_in.trim_start_matches("0x"));
    let route_to = format!("{:0>64}", token_out.trim_start_matches("0x"));
    let route_stable = format!("{:0>64x}", stable as u64);
    let route_factory = format!("{:0>64}", factory.trim_start_matches("0x"));

    let data = format!(
        "0x5509a1ac{}{}{}{}{}{}{}",
        amount_in_hex,
        routes_offset,
        routes_length,
        route_from,
        route_to,
        route_stable,
        route_factory,
    );

    let hex = eth_call(router, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");

    // Returns uint256[] -- ABI: offset(32) + length(32) + amounts[0](32) + amounts[1](32)
    if clean.len() < 192 {
        anyhow::bail!("getAmountsOut: unexpected response length");
    }
    let word3 = &clean[192..256.min(clean.len())];
    let trimmed = if word3.len() > 32 { &word3[word3.len() - 32..] } else { word3 };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// Router.quoteAddLiquidity(address tokenA, address tokenB, bool stable, address _factory,
///   uint256 amountADesired, uint256 amountBDesired)
/// -> (uint256 amountA, uint256 amountB, uint256 liquidity)
/// Selector: 0xce700c29
pub async fn router_quote_add_liquidity(
    router: &str,
    token_a: &str,
    token_b: &str,
    stable: bool,
    factory: &str,
    amount_a: u128,
    amount_b: u128,
    rpc_url: &str,
) -> anyhow::Result<(u128, u128, u128)> {
    let ta = format!("{:0>64}", token_a.trim_start_matches("0x"));
    let tb = format!("{:0>64}", token_b.trim_start_matches("0x"));
    let s = format!("{:0>64x}", stable as u64);
    let f = format!("{:0>64}", factory.trim_start_matches("0x"));
    let aa = format!("{:0>64x}", amount_a);
    let ab = format!("{:0>64x}", amount_b);
    let data = format!("0xce700c29{}{}{}{}{}{}", ta, tb, s, f, aa, ab);
    let hex = eth_call(router, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");

    let parse_word = |i: usize| -> u128 {
        let start = i * 64;
        let end = start + 64;
        if end > clean.len() { return 0; }
        let w = &clean[start..end];
        let t = if w.len() > 32 { &w[w.len() - 32..] } else { w };
        u128::from_str_radix(t, 16).unwrap_or(0)
    };

    Ok((parse_word(0), parse_word(1), parse_word(2)))
}

/// Pool.getReserves() -> (uint256 reserve0, uint256 reserve1, uint256 blockTimestampLast)
/// Selector: 0x0902f1ac
pub async fn pool_get_reserves(pool: &str, rpc_url: &str) -> anyhow::Result<(u128, u128)> {
    let data = "0x0902f1ac";
    let hex = eth_call(pool, data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");

    let parse_word = |i: usize| -> u128 {
        let start = i * 64;
        let end = start + 64;
        if end > clean.len() { return 0; }
        let w = &clean[start..end];
        let t = if w.len() > 32 { &w[w.len() - 32..] } else { w };
        u128::from_str_radix(t, 16).unwrap_or(0)
    };

    Ok((parse_word(0), parse_word(1)))
}

/// Pool.totalSupply() -> uint256
/// Selector: 0x18160ddd
pub async fn pool_total_supply(pool: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = "0x18160ddd";
    let hex = eth_call(pool, data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// Pool.token0() -> address
/// Selector: 0x0dfe1681
pub async fn pool_token0(pool: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(pool, "0x0dfe1681", rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    Ok(if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    })
}

/// Pool.token1() -> address
/// Selector: 0xd21220a7
pub async fn pool_token1(pool: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(pool, "0xd21220a7", rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    Ok(if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    })
}

/// Voter.gauges(address pool) -> address gauge
/// Selector: 0xb9a09fd5
pub async fn voter_get_gauge(voter: &str, pool: &str, rpc_url: &str) -> anyhow::Result<String> {
    let pool_padded = format!("{:0>64}", pool.trim_start_matches("0x"));
    let data = format!("0xb9a09fd5{}", pool_padded);
    let hex = eth_call(voter, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    Ok(if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    })
}

/// Gauge.earned(address account) -> uint256
/// Selector: 0x008cc262
pub async fn gauge_earned(gauge: &str, account: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let acct = format!("{:0>64}", account.trim_start_matches("0x"));
    let data = format!("0x008cc262{}", acct);
    let hex = eth_call(gauge, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
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

/// ERC-20 decimals() → u8. Falls back to 18 on error.
pub async fn get_erc20_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let result = eth_call(token, "0x313ce567", rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 2 { return Ok(18); }
    Ok(u8::from_str_radix(&clean[clean.len() - 2..], 16).unwrap_or(18))
}
