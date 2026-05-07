/// RPC helpers: eth_call, QuoterV2, factory pool queries, token metadata.

use anyhow::Result;
use serde_json::json;

// ── Raw eth_call ──────────────────────────────────────────────────────────────

/// Extract a human-readable error string from an eth_call JSON error object.
/// Strips the raw ABI-encoded hex suffix that some RPC nodes append.
/// e.g. `{"code":3,"message":"execution reverted: Foo: 0x08c379a0..."}` → "execution reverted: Foo"
fn decode_rpc_error(err: &serde_json::Value) -> String {
    if let Some(msg) = err["message"].as_str() {
        // Some RPC nodes append `: 0x<abidata>` after the revert reason — strip it
        if let Some(idx) = msg.find(": 0x") {
            return msg[..idx].to_string();
        }
        return msg.to_string();
    }
    err.to_string()
}

/// Execute an eth_call and return the hex result string.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> Result<String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": data }, "latest"],
        "id": 1
    });
    let resp: serde_json::Value = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", decode_rpc_error(err));
    }

    let result = resp["result"].as_str().ok_or_else(|| {
        anyhow::anyhow!("eth_call: malformed RPC response (missing 'result' field): {}", resp)
    })?;
    Ok(result.to_string())
}

/// Execute an eth_call with an explicit gas limit (needed for QuoterV2 simulation).
pub async fn eth_call_with_gas(to: &str, data: &str, rpc_url: &str, gas: &str) -> Result<String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": data, "gas": gas }, "latest"],
        "id": 1
    });
    let resp: serde_json::Value = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", decode_rpc_error(err));
    }

    let result = resp["result"].as_str().ok_or_else(|| {
        anyhow::anyhow!("eth_call_with_gas: malformed RPC response (missing 'result' field): {}", resp)
    })?;
    Ok(result.to_string())
}

// ── Hex decode helpers ────────────────────────────────────────────────────────

pub fn decode_u256_from_hex(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    // Pad to exactly 64 hex chars (32 bytes = U256)
    let padded = format!("{:0>64}", clean);
    // Upper 32 hex chars (16 bytes) represent the high 128 bits of the U256
    let upper = &padded[..32];
    if upper.chars().any(|c| c != '0') {
        // Value exceeds u128::MAX — clamp with warning
        eprintln!("Warning: on-chain value exceeds u128::MAX, clamping");
        return u128::MAX;
    }
    // Lower 32 hex chars hold the u128 value
    let lower = &padded[32..];
    u128::from_str_radix(lower, 16).unwrap_or(0)
}

pub fn decode_address_from_hex(hex: &str) -> String {
    let raw = hex.trim_start_matches("0x");
    if raw.len() >= 40 {
        format!("0x{}", &raw[raw.len() - 40..])
    } else {
        format!("0x{:0>40}", raw)
    }
}

// ── Token metadata ────────────────────────────────────────────────────────────

/// Get ERC-20 decimals via eth_call.
pub async fn get_decimals(token: &str, rpc_url: &str) -> Result<u8> {
    // decimals() selector = 0x313ce567
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    let raw = hex.trim_start_matches("0x");
    Ok(u8::from_str_radix(&raw[raw.len().saturating_sub(2)..], 16).unwrap_or(18))
}

/// Get ERC-20 symbol via eth_call (returns UTF-8 decoded string).
pub async fn get_symbol(token: &str, rpc_url: &str) -> Result<String> {
    // symbol() selector = 0x95d89b41
    let hex = eth_call(token, "0x95d89b41", rpc_url).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 {
        return Ok(format!("0x{}", &token[2..6]));
    }
    // ABI-encoded string: offset (32 bytes) + length (32 bytes) + data
    // length is at bytes 32-64 (chars 64-128)
    let len_hex = &raw[64..128];
    let len = usize::from_str_radix(len_hex, 16).unwrap_or(0);
    let data_hex = &raw[128..128 + len * 2];
    let bytes = hex::decode(data_hex).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Get ERC-20 allowance via eth_call.
/// allowance(address owner, address spender) selector = 0xdd62ed3e
pub async fn get_allowance(token: &str, owner: &str, spender: &str, rpc_url: &str) -> Result<u128> {
    let padded_owner = format!("{:0>64}", &owner[2..]);
    let padded_spender = format!("{:0>64}", &spender[2..]);
    let hex = eth_call(token, &format!("0xdd62ed3e{}{}", padded_owner, padded_spender), rpc_url).await?;
    Ok(decode_u256_from_hex(&hex))
}

/// Get ERC-20 balance via eth_call.
pub async fn get_balance(token: &str, account: &str, rpc_url: &str) -> Result<u128> {
    // balanceOf(address) = 0x70a08231
    let padded = format!("{:0>64}", &account[2..]);
    let hex = eth_call(token, &format!("0x70a08231{}", padded), rpc_url).await?;
    Ok(decode_u256_from_hex(&hex))
}

/// Fetch the native coin balance (BNB/ETH) of an address via eth_getBalance.
pub async fn get_native_balance(account: &str, rpc_url: &str) -> Result<u128> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_getBalance",
        "params": [account, "latest"],
        "id": 1
    });
    let resp: serde_json::Value = reqwest::Client::new()
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_getBalance error: {}", err);
    }
    Ok(decode_u256_from_hex(resp["result"].as_str().unwrap_or("0x0")))
}

/// Count LP positions owned by address via NonfungiblePositionManager.balanceOf.
pub async fn get_lp_position_count(npm: &str, owner: &str, rpc_url: &str) -> Result<usize> {
    let padded = format!("{:0>64}", &owner[2..]);
    let hex = eth_call(npm, &format!("0x70a08231{}", padded), rpc_url).await?;
    Ok(decode_u256_from_hex(&hex) as usize)
}

// ── PancakeV3Factory ──────────────────────────────────────────────────────────

/// Get pool address from factory.
/// getPool(address,address,uint24) selector = 0x1698ee82
pub async fn get_pool_address(
    factory: &str,
    token_a: &str,
    token_b: &str,
    fee: u32,
    rpc_url: &str,
) -> Result<String> {
    use alloy_primitives::Address;
    // encode: address (32 bytes), address (32 bytes), uint24 (32 bytes)
    let addr_a: Address = token_a.parse()?;
    let addr_b: Address = token_b.parse()?;
    let calldata = format!(
        "0x1698ee82{:0>64}{:0>64}{:0>64}",
        hex::encode(addr_a.as_slice()),
        hex::encode(addr_b.as_slice()),
        format!("{:x}", fee)
    );
    let result = eth_call(factory, &calldata, rpc_url).await?;
    let pool = decode_address_from_hex(&result);
    if pool == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("No pool found for this token pair and fee tier");
    }
    Ok(pool)
}

// ── Pool queries ──────────────────────────────────────────────────────────────

/// Query slot0 from pool contract.
/// slot0() selector = 0x3850c7bd
/// Returns (sqrtPriceX96, tick, observationIndex, observationCardinality, observationCardinalityNext, feeProtocol, unlocked)
pub async fn get_slot0(pool: &str, rpc_url: &str) -> Result<(u128, i32)> {
    let hex = eth_call(pool, "0x3850c7bd", rpc_url).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 {
        anyhow::bail!("Invalid slot0 response from pool {}", pool);
    }
    let sqrt_price_hex = &raw[0..64];
    let tick_hex = &raw[64..128];

    let sqrt_price = decode_u256_from_hex(sqrt_price_hex);

    // tick is int24, ABI-padded to 32 bytes (64 hex chars) as int256.
    // Negative ticks have all high bytes set to 0xFF — u128::from_str_radix
    // would overflow and fall back to 0.  Decode only the last 8 hex chars
    // (32 bits) and reinterpret as i32, exactly like decode_int24_from_field
    // in get_position().
    let low32 = u32::from_str_radix(&tick_hex[56..64], 16).unwrap_or(0);
    let tick = low32 as i32;

    Ok((sqrt_price, tick))
}

/// Query liquidity() from pool contract.
/// liquidity() selector = 0x1a686502
pub async fn get_pool_liquidity(pool: &str, rpc_url: &str) -> Result<u128> {
    let hex = eth_call(pool, "0x1a686502", rpc_url).await?;
    Ok(decode_u256_from_hex(&hex))
}

// ── QuoterV2 ──────────────────────────────────────────────────────────────────

/// Quote exact input single via QuoterV2 eth_call.
/// Uses ~5M gas limit to avoid false out-of-gas from the simulation.
pub async fn quote_exact_input_single(
    quoter: &str,
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    fee: u32,
    rpc_url: &str,
) -> Result<u128> {
    use crate::calldata::encode_quote_exact_input_single;
    let calldata = encode_quote_exact_input_single(token_in, token_out, amount_in, fee)?;
    // Use 0x4C4B40 (~5M gas) as required by QuoterV2 simulation
    let result = eth_call_with_gas(quoter, &calldata, rpc_url, "0x4C4B40").await?;
    let raw = result.trim_start_matches("0x");
    if raw.len() < 64 {
        anyhow::bail!("QuoterV2 returned empty/short result — pool may not exist or fee tier mismatch");
    }
    // amountOut is the first 32 bytes of the return
    let amount_out = decode_u256_from_hex(&raw[0..64]);
    Ok(amount_out)
}

// ── NonfungiblePositionManager ────────────────────────────────────────────────

/// Query positions(tokenId) from NonfungiblePositionManager.
/// Returns simplified struct with key fields.
pub struct PositionData {
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub tokens_owed0: u128,
    pub tokens_owed1: u128,
}

/// positions(uint256) selector = 0x99fbab88
pub async fn get_position(npm: &str, token_id: u128, rpc_url: &str) -> Result<PositionData> {
    let calldata = format!("0x99fbab88{:0>64x}", token_id);
    let hex = eth_call(npm, &calldata, rpc_url).await?;
    let raw = hex.trim_start_matches("0x");

    // Each field is 32 bytes = 64 hex chars
    // Fields: nonce(0), operator(1), token0(2), token1(3), fee(4), tickLower(5), tickUpper(6),
    //         liquidity(7), feeGrowthInside0LastX128(8), feeGrowthInside1LastX128(9),
    //         tokensOwed0(10), tokensOwed1(11)
    if raw.len() < 12 * 64 {
        anyhow::bail!("Invalid positions() response for tokenId {}", token_id);
    }

    let field = |n: usize| &raw[n * 64..(n + 1) * 64];

    let token0 = decode_address_from_hex(field(2));
    let token1 = decode_address_from_hex(field(3));
    let fee = u32::from_str_radix(field(4), 16).unwrap_or(0);

    // tick fields are ABI-encoded as int256 (64 hex chars / 256 bits).
    // For negative ticks, the upper bits are all 1s (sign extension).
    // We decode the lower 32 bits as i32, reading the last 8 hex chars.
    let decode_int24_from_field = |s: &str| -> i32 {
        // s is 64 hex chars; take the last 8 (= 32-bit value)
        let low32 = u32::from_str_radix(&s[s.len()-8..], 16).unwrap_or(0);
        low32 as i32
    };
    let tick_lower: i32 = decode_int24_from_field(field(5));
    let tick_upper: i32 = decode_int24_from_field(field(6));

    let liquidity = u128::from_str_radix(field(7), 16).unwrap_or(0);
    let tokens_owed0 = u128::from_str_radix(field(10), 16).unwrap_or(0);
    let tokens_owed1 = u128::from_str_radix(field(11), 16).unwrap_or(0);

    Ok(PositionData {
        token0,
        token1,
        fee,
        tick_lower,
        tick_upper,
        liquidity,
        tokens_owed0,
        tokens_owed1,
    })
}

/// balanceOf(address) and tokenOfOwnerByIndex(address,uint256) for NPM enumeration.
pub async fn get_token_ids_for_owner(
    npm: &str,
    owner: &str,
    rpc_url: &str,
) -> Result<Vec<u128>> {
    // balanceOf(address) = 0x70a08231
    let padded_owner = format!("{:0>64}", &owner[2..]);
    let balance_hex = eth_call(npm, &format!("0x70a08231{}", padded_owner), rpc_url).await?;
    let balance = decode_u256_from_hex(&balance_hex) as usize;

    const MAX_POSITIONS: usize = 100;
    let capped = balance.min(MAX_POSITIONS);
    if balance > MAX_POSITIONS {
        eprintln!(
            "WARNING: address holds {} positions; showing first {} only.",
            balance, MAX_POSITIONS
        );
    }

    let mut ids = Vec::with_capacity(capped);
    for i in 0..capped {
        // tokenOfOwnerByIndex(address,uint256) = 0x2f745c59
        let calldata = format!(
            "0x2f745c59{:0>64}{:0>64x}",
            &owner[2..],
            i
        );
        let hex = eth_call(npm, &calldata, rpc_url).await?;
        ids.push(decode_u256_from_hex(&hex));
    }
    Ok(ids)
}

// ── V3 liquidity math ─────────────────────────────────────────────────────────

/// Compute the actual amounts that will be deposited when minting a V3 position.
///
/// V3 deposits the optimal ratio for the current price — NOT necessarily the full
/// desired amounts. One token will be fully consumed; the other may be partially used.
/// Slippage minimums must be applied to THESE actual amounts, not to desired amounts.
///
/// Algorithm mirrors NonfungiblePositionManager._addLiquidity():
///   L = min(L_from_amount0, L_from_amount1)
///   actual amounts are then re-derived from L and current sqrtPrice.
pub fn amounts_for_add_liquidity(
    sqrt_price_x96: u128,
    tick_lower: i32,
    tick_upper: i32,
    tick_current: i32,
    amount0_desired: u128,
    amount1_desired: u128,
) -> (u128, u128) {
    let sqrt_p = sqrt_price_x96 as f64 / (1u128 << 96) as f64;
    let sqrt_a = tick_to_sqrt_price(tick_lower);
    let sqrt_b = tick_to_sqrt_price(tick_upper);

    if tick_current < tick_lower {
        // Position entirely in token0
        (amount0_desired, 0)
    } else if tick_current >= tick_upper {
        // Position entirely in token1
        (0, amount1_desired)
    } else {
        // In-range: compute L from each desired amount, take min
        let l_from_0 = amount0_desired as f64 * sqrt_p * sqrt_b / (sqrt_b - sqrt_p);
        let l_from_1 = amount1_desired as f64 / (sqrt_p - sqrt_a);
        let l = l_from_0.min(l_from_1);

        let actual0 = l * (sqrt_b - sqrt_p) / (sqrt_p * sqrt_b);
        let actual1 = l * (sqrt_p - sqrt_a);
        (actual0 as u128, actual1 as u128)
    }
}

/// Compute the actual token amounts held by a V3 position given the current pool price.
///
/// Uses f64 arithmetic (sufficient for slippage bound estimation — we only need
/// ~1% accuracy, not wei-exact values).
///
/// Formula (from Uniswap V3 whitepaper):
///   if tick < tickLower  → all token0: amount0 = L·(√B − √A) / (√A·√B)
///   if tick ≥ tickUpper  → all token1: amount1 = L·(√B − √A)
///   in range             → amount0 = L·(√B − √P) / (√P·√B)
///                          amount1 = L·(√P − √A)
///
/// Returns (amount0, amount1) in minimal units (wei).
pub fn amounts_from_liquidity(
    sqrt_price_x96: u128,
    tick_lower: i32,
    tick_upper: i32,
    tick_current: i32,
    liquidity: u128,
) -> (u128, u128) {
    let q96 = (1u128 << 96) as f64;
    let sqrt_p = sqrt_price_x96 as f64 / q96;
    let sqrt_a = tick_to_sqrt_price(tick_lower);
    let sqrt_b = tick_to_sqrt_price(tick_upper);
    let liq = liquidity as f64;

    if tick_current < tick_lower {
        let amount0 = liq * (sqrt_b - sqrt_a) / (sqrt_a * sqrt_b);
        (amount0 as u128, 0)
    } else if tick_current >= tick_upper {
        let amount1 = liq * (sqrt_b - sqrt_a);
        (0, amount1 as u128)
    } else {
        let amount0 = liq * (sqrt_b - sqrt_p) / (sqrt_p * sqrt_b);
        let amount1 = liq * (sqrt_p - sqrt_a);
        (amount0 as u128, amount1 as u128)
    }
}

/// sqrt(1.0001^tick) — the Q96 sqrt price at a given tick, as a plain f64.
fn tick_to_sqrt_price(tick: i32) -> f64 {
    f64::powf(1.0001_f64, tick as f64 / 2.0)
}

// ── Subgraph ──────────────────────────────────────────────────────────────────

/// Query LP positions from TheGraph subgraph.
pub async fn query_positions_subgraph(
    subgraph_url: &str,
    owner: &str,
) -> Result<serde_json::Value> {
    let query = format!(
        r#"{{
  "query": "{{ positions(where: {{ owner: \"{}\", liquidity_gt: \"0\" }}) {{ id token0 {{ symbol decimals }} token1 {{ symbol decimals }} feeTier tickLower {{ tickIdx }} tickUpper {{ tickIdx }} liquidity depositedToken0 depositedToken1 collectedFeesToken0 collectedFeesToken1 }} }}"
}}"#,
        owner.to_lowercase()
    );

    let resp: serde_json::Value = reqwest::Client::new()
        .post(subgraph_url)
        .header("Content-Type", "application/json")
        .body(query)
        .send()
        .await?
        .json()
        .await?;

    Ok(resp)
}
