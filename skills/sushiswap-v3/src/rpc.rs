use anyhow::Context;
use serde_json::{json, Value};

/// Perform an eth_call via JSON-RPC and return the raw hex result.
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

/// Get ERC-20 token balance.
/// balanceOf(address) → uint256
/// Selector: 0x70a08231
#[allow(dead_code)]
pub async fn get_balance(token: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let data = format!("0x70a08231{}", owner_padded);
    let hex = eth_call(token, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// SushiSwap V3 Factory.getPool(address,address,uint24) → address
/// Selector: 0x1698ee82
pub async fn factory_get_pool(
    token0: &str,
    token1: &str,
    fee: u32,
    factory: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let t0 = format!("{:0>64}", token0.trim_start_matches("0x"));
    let t1 = format!("{:0>64}", token1.trim_start_matches("0x"));
    let fee_hex = format!("{:0>64x}", fee);
    let data = format!("0x1698ee82{}{}{}", t0, t1, fee_hex);
    let hex = eth_call(factory, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let addr = if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    };
    Ok(addr)
}

/// NonfungiblePositionManager.balanceOf(address) → uint256
/// Selector: 0x70a08231
pub async fn nfpm_balance_of(nfpm: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u64> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let data = format!("0x70a08231{}", owner_padded);
    let hex = eth_call(nfpm, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 16 { &clean[clean.len() - 16..] } else { clean };
    Ok(u64::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// NonfungiblePositionManager.tokenOfOwnerByIndex(address,uint256) → uint256
/// Selector: 0x2f745c59
pub async fn nfpm_token_of_owner_by_index(
    nfpm: &str,
    owner: &str,
    index: u64,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let index_hex = format!("{:0>64x}", index);
    let data = format!("0x2f745c59{}{}", owner_padded, index_hex);
    let hex = eth_call(nfpm, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    let trimmed = if clean.len() > 32 { &clean[clean.len() - 32..] } else { clean };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}

/// Decoded position returned by NonfungiblePositionManager.positions(tokenId).
#[derive(Debug)]
pub struct PositionData {
    #[allow(dead_code)]
    pub token_id: u128,
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub tokens_owed0: u128,
    pub tokens_owed1: u128,
}

/// NonfungiblePositionManager.positions(uint256)
/// Returns: (nonce, operator, token0, token1, fee, tickLower, tickUpper,
///           liquidity, feeGrowthInside0LastX128, feeGrowthInside1LastX128,
///           tokensOwed0, tokensOwed1)
/// Selector: 0x99fbab88
pub async fn nfpm_positions(
    nfpm: &str,
    token_id: u128,
    rpc_url: &str,
) -> anyhow::Result<PositionData> {
    let token_id_hex = format!("{:0>64x}", token_id);
    let data = format!("0x99fbab88{}", token_id_hex);
    let hex = eth_call(nfpm, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");

    // ABI words: each is 64 hex chars (32 bytes).
    // Word 0: nonce, 1: operator, 2: token0, 3: token1, 4: fee, 5: tickLower, 6: tickUpper,
    // 7: liquidity, 8: feeGrowth0, 9: feeGrowth1, 10: tokensOwed0, 11: tokensOwed1
    let words: Vec<&str> = (0..12)
        .map(|i| {
            let start = i * 64;
            let end = start + 64;
            if end <= clean.len() { &clean[start..end] } else { "0" }
        })
        .collect();

    let parse_addr = |w: &str| -> String {
        if w.len() >= 40 {
            format!("0x{}", &w[w.len() - 40..])
        } else {
            "0x0000000000000000000000000000000000000000".to_string()
        }
    };
    let parse_u32 = |w: &str| -> u32 {
        let last8 = if w.len() >= 8 { &w[w.len() - 8..] } else { w };
        u32::from_str_radix(last8, 16).unwrap_or(0)
    };
    let parse_tick = |w: &str| -> i32 {
        let last8 = if w.len() >= 8 { &w[w.len() - 8..] } else { w };
        u32::from_str_radix(last8, 16).unwrap_or(0) as i32
    };
    let parse_u128 = |w: &str| -> u128 {
        let trimmed = if w.len() > 32 { &w[w.len() - 32..] } else { w };
        u128::from_str_radix(trimmed, 16).unwrap_or(0)
    };

    Ok(PositionData {
        token_id,
        token0: parse_addr(words[2]),
        token1: parse_addr(words[3]),
        fee: parse_u32(words[4]),
        tick_lower: parse_tick(words[5]),
        tick_upper: parse_tick(words[6]),
        liquidity: parse_u128(words[7]),
        tokens_owed0: parse_u128(words[10]),
        tokens_owed1: parse_u128(words[11]),
    })
}

/// QuoterV2.quoteExactInputSingle(QuoteExactInputSingleParams)
/// Params: (address tokenIn, address tokenOut, uint256 amountIn, uint24 fee, uint160 sqrtPriceLimitX96)
/// Returns: (uint256 amountOut, uint160 sqrtPriceX96After, uint32 initializedTicksCrossed, uint256 gasEstimate)
/// Selector: 0xc6a5026a
pub async fn quoter_exact_input_single(
    quoter: &str,
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    fee: u32,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let t_in = format!("{:0>64}", token_in.trim_start_matches("0x"));
    let t_out = format!("{:0>64}", token_out.trim_start_matches("0x"));
    let amt = format!("{:0>64x}", amount_in);
    let fee_hex = format!("{:0>64x}", fee);
    let sqrt_limit = "0".repeat(64); // sqrtPriceLimitX96 = 0 means no price limit
    let data = format!("0xc6a5026a{}{}{}{}{}", t_in, t_out, amt, fee_hex, sqrt_limit);
    let hex = eth_call(quoter, &data, rpc_url).await?;
    let clean = hex.trim_start_matches("0x");
    // First 64 hex chars = first ABI word = amountOut
    let first_word = if clean.len() >= 64 { &clean[..64] } else { clean };
    let trimmed = if first_word.len() > 32 { &first_word[first_word.len() - 32..] } else { first_word };
    Ok(u128::from_str_radix(trimmed, 16).unwrap_or(0))
}
