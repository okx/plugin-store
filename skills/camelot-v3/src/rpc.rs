use reqwest::Client;
use serde_json::{json, Value};

/// Low-level eth_call via JSON-RPC.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    let result = resp["result"].as_str().unwrap_or("0x").to_string();
    Ok(result)
}

/// AlgebraFactory.poolByPair(address,address) — selector 0xd9a641e1
/// Returns pool address. Zero address = pool not deployed.
pub async fn factory_pool_by_pair(
    token_a: &str,
    token_b: &str,
    factory: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let a_padded = format!("{:0>64}", token_a.trim_start_matches("0x"));
    let b_padded = format!("{:0>64}", token_b.trim_start_matches("0x"));
    let data = format!("0xd9a641e1{}{}", a_padded, b_padded);
    let result = eth_call(factory, &data, rpc_url).await?;
    // Result is a 32-byte padded address — extract last 40 chars
    let clean = result.trim_start_matches("0x");
    if clean.len() < 40 {
        return Ok("0x0000000000000000000000000000000000000000".to_string());
    }
    Ok(format!("0x{}", &clean[clean.len() - 40..]))
}

/// Quoter.quoteExactInputSingle(address,address,uint256,uint160) — selector 0x2d9ebd1d
/// Algebra V1: no fee tier, limitSqrtPrice=0 for no limit.
/// Returns amountOut (first 32 bytes of result).
pub async fn quoter_exact_input_single(
    quoter: &str,
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let token_in_padded = format!("{:0>64}", token_in.trim_start_matches("0x"));
    let token_out_padded = format!("{:0>64}", token_out.trim_start_matches("0x"));
    let amount_in_hex = format!("{:0>64x}", amount_in);
    // limitSqrtPrice = 0
    let limit_sqrt = format!("{:0>64x}", 0u128);
    let data = format!(
        "0x2d9ebd1d{}{}{}{}",
        token_in_padded, token_out_padded, amount_in_hex, limit_sqrt
    );
    let result = eth_call(quoter, &data, rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 64 {
        anyhow::bail!("Quoter returned short result: {}", result);
    }
    // First 32 bytes = amountOut
    let amount_out_hex = &clean[..64];
    let amount_out = u128::from_str_radix(amount_out_hex, 16)
        .map_err(|_| anyhow::anyhow!("Failed to parse amountOut: {}", amount_out_hex))?;
    Ok(amount_out)
}

/// ERC-20 allowance(address,address) — selector 0xdd62ed3e
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
    if clean.is_empty() {
        return Ok(0);
    }
    Ok(u128::from_str_radix(clean, 16).unwrap_or(0))
}

/// ERC-20 decimals() — selector 0x313ce567
pub async fn get_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let data = "0x313ce567";
    let hex = eth_call(token, data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let val = u64::from_str_radix(clean, 16).unwrap_or(18);
    Ok(val as u8)
}

/// ERC-20 symbol() — selector 0x95d89b41 (returns ABI-encoded string)
pub async fn get_symbol(token: &str, rpc_url: &str) -> anyhow::Result<String> {
    let data = "0x95d89b41";
    let hex = eth_call(token, data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    // ABI-encoded string: offset(32) + length(32) + data
    if clean.len() < 128 {
        return Ok("UNKNOWN".to_string());
    }
    let len_hex = &clean[64..128];
    let len = usize::from_str_radix(len_hex, 16).unwrap_or(0);
    let data_start = 128;
    let data_end = data_start + len * 2;
    if data_end > clean.len() {
        return Ok("UNKNOWN".to_string());
    }
    let symbol_hex = &clean[data_start..data_end];
    let bytes = hex::decode(symbol_hex).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// NFPM balanceOf(address) — returns number of NFT positions owned
pub async fn nfpm_balance_of(nfpm: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u64> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let data = format!("0x70a08231{}", owner_padded);
    let hex = eth_call(nfpm, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    Ok(u64::from_str_radix(clean, 16).unwrap_or(0))
}

/// NFPM tokenOfOwnerByIndex(address,uint256) — selector: keccak("tokenOfOwnerByIndex(address,uint256)") = 0x2f745c59
pub async fn nfpm_token_of_owner_by_index(
    nfpm: &str,
    owner: &str,
    index: u64,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let index_padded = format!("{:0>64x}", index);
    let data = format!("0x2f745c59{}{}", owner_padded, index_padded);
    let hex = eth_call(nfpm, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    Ok(u128::from_str_radix(clean, 16).unwrap_or(0))
}

/// NFPM positions(uint256 tokenId) — selector 0x99fbab88
/// Returns: nonce, operator, token0, token1, tickLower, tickUpper, liquidity, ...
pub async fn nfpm_positions(
    nfpm: &str,
    token_id: u128,
    rpc_url: &str,
) -> anyhow::Result<Value> {
    let token_id_hex = format!("{:0>64x}", token_id);
    let data = format!("0x99fbab88{}", token_id_hex);
    let result = eth_call(nfpm, &data, rpc_url).await?;
    let clean = result.trim_start_matches("0x");

    // Parse 32-byte chunks
    let chunks: Vec<&str> = (0..clean.len())
        .step_by(64)
        .filter(|&i| i + 64 <= clean.len())
        .map(|i| &clean[i..i + 64])
        .collect();

    if chunks.len() < 9 {
        anyhow::bail!("positions() returned short result: {} chunks", chunks.len());
    }

    // chunk[0] = nonce (uint96)
    // chunk[1] = operator (address, last 40 chars)
    // chunk[2] = token0 (address)
    // chunk[3] = token1 (address)
    // chunk[4] = tickLower (int24)
    // chunk[5] = tickUpper (int24)
    // chunk[6] = liquidity (uint128)
    // chunk[7] = feeGrowthInside0LastX128
    // chunk[8] = feeGrowthInside1LastX128
    // chunk[9] = tokensOwed0 (uint128)
    // chunk[10] = tokensOwed1 (uint128)

    fn decode_tick_from_chunk(chunk: &str) -> i32 {
        let last8 = &chunk[chunk.len().saturating_sub(8)..];
        u32::from_str_radix(last8, 16).unwrap_or(0) as i32
    }

    let token0 = format!("0x{}", &chunks[2][24..]);
    let token1 = format!("0x{}", &chunks[3][24..]);
    let tick_lower = decode_tick_from_chunk(chunks[4]);
    let tick_upper = decode_tick_from_chunk(chunks[5]);
    let liquidity = u128::from_str_radix(chunks[6], 16).unwrap_or(0);
    let tokens_owed0 = if chunks.len() > 9 {
        u128::from_str_radix(chunks[9], 16).unwrap_or(0)
    } else {
        0
    };
    let tokens_owed1 = if chunks.len() > 10 {
        u128::from_str_radix(chunks[10], 16).unwrap_or(0)
    } else {
        0
    };

    Ok(serde_json::json!({
        "token_id": token_id,
        "token0": token0,
        "token1": token1,
        "tick_lower": tick_lower,
        "tick_upper": tick_upper,
        "liquidity": liquidity.to_string(),
        "tokens_owed0": tokens_owed0.to_string(),
        "tokens_owed1": tokens_owed1.to_string()
    }))
}
