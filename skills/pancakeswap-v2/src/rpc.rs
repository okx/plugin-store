use anyhow::Context;
use serde_json::{json, Value};

/// Perform an eth_call via JSON-RPC and return the hex result string.
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
/// allowance(address owner, address spender) → uint256
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
/// balanceOf(address) → uint256
/// Selector: 0x70a08231
pub async fn get_balance(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let data = format!("0x70a08231{}", owner_padded);
    let hex = eth_call(token, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// ERC-20 totalSupply() → uint256
/// Selector: 0x18160ddd
pub async fn get_total_supply(token: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = "0x18160ddd";
    let hex = eth_call(token, data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// Factory.getPair(address tokenA, address tokenB) → address
/// Selector: 0xe6a43905
pub async fn factory_get_pair(
    factory: &str,
    token_a: &str,
    token_b: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let ta = format!("{:0>64}", token_a.trim_start_matches("0x"));
    let tb = format!("{:0>64}", token_b.trim_start_matches("0x"));
    let data = format!("0xe6a43905{}{}", ta, tb);
    let hex = eth_call(factory, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let addr = if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    };
    Ok(addr)
}

/// Pair.getReserves() → (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
/// Selector: 0x0902f1ac
pub async fn get_reserves(pair: &str, rpc_url: &str) -> anyhow::Result<(u128, u128)> {
    let data = "0x0902f1ac";
    let hex = eth_call(pair, data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    // Returns 3 packed ABI words (96 bytes = 192 hex chars)
    // reserve0 is word 0 (first 64 chars), reserve1 is word 1 (next 64 chars)
    let r0_hex = if clean.len() >= 64 { &clean[..64] } else { "0" };
    let r1_hex = if clean.len() >= 128 { &clean[64..128] } else { "0" };
    // Take last 28 chars (14 bytes = 112 bits) for uint112
    let r0_trimmed = if r0_hex.len() > 28 { &r0_hex[r0_hex.len() - 28..] } else { r0_hex };
    let r1_trimmed = if r1_hex.len() > 28 { &r1_hex[r1_hex.len() - 28..] } else { r1_hex };
    let r0 = u128::from_str_radix(r0_trimmed, 16).unwrap_or(0);
    let r1 = u128::from_str_radix(r1_trimmed, 16).unwrap_or(0);
    Ok((r0, r1))
}

/// Pair.token0() → address
/// Selector: 0x0dfe1681
pub async fn get_token0(pair: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(pair, "0x0dfe1681", rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let addr = if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    };
    Ok(addr)
}

/// Router.getAmountsOut(uint256 amountIn, address[] path) → uint256[]
/// Selector: 0xd06ca61f
/// Returns the output amounts array; last element is the final output amount.
pub async fn get_amounts_out(
    router: &str,
    amount_in: u128,
    path: &[&str],
    rpc_url: &str,
) -> anyhow::Result<Vec<u128>> {
    // ABI encoding:
    // - amountIn: uint256 (word 0)
    // - offset to path array: 0x40 = 64 (word 1) — two fixed params before dynamic
    // - path.length (word 2)
    // - path[0..N] (words 3..3+N)
    let amount_in_hex = format!("{:0>64x}", amount_in);
    let offset_hex = format!("{:0>64x}", 0x40usize);
    let len_hex = format!("{:0>64x}", path.len());
    let mut elems = String::new();
    for addr in path {
        elems.push_str(&format!("{:0>64}", addr.trim_start_matches("0x")));
    }
    let data = format!("0xd06ca61f{}{}{}{}", amount_in_hex, offset_hex, len_hex, elems);
    let hex = eth_call(router, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    // Returns: offset(word0), length(word1), amounts(word2..2+N)
    // We need to skip the offset and length words
    let mut amounts = Vec::new();
    if clean.len() >= 128 {
        let length_hex = &clean[64..128];
        let length = usize::from_str_radix(length_hex, 16).unwrap_or(0);
        for i in 0..length {
            let start = 128 + i * 64;
            let end = start + 64;
            if end <= clean.len() {
                let word = &clean[start..end];
                let trimmed = if word.len() > 32 { &word[word.len() - 32..] } else { word };
                amounts.push(u128::from_str_radix(trimmed, 16).unwrap_or(0));
            }
        }
    }
    Ok(amounts)
}
