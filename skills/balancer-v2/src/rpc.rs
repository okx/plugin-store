/// Direct eth_call helpers (no onchainos needed for reads)

use anyhow::Result;
use serde_json::{json, Value};

pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> Result<String> {
    let client = reqwest::Client::new();
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
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a uint256 from 32-byte hex
pub fn decode_u256_hex(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return 0;
    }
    // Take last 32 hex chars (16 bytes = u128 range covers most values)
    let last32 = &clean[clean.len().saturating_sub(32)..];
    u128::from_str_radix(last32, 16).unwrap_or(0)
}

/// Decode uint256 as u64 (for values that fit)
pub fn decode_u64_hex(hex: &str) -> u64 {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 16 {
        return 0;
    }
    let last16 = &clean[clean.len().saturating_sub(16)..];
    u64::from_str_radix(last16, 16).unwrap_or(0)
}

/// Decode an EVM address from 32-byte ABI-encoded slot
pub fn decode_address(slot: &str) -> String {
    let clean = slot.trim_start_matches("0x");
    if clean.len() < 40 {
        return "0x0000000000000000000000000000000000000000".to_string();
    }
    format!("0x{}", &clean[clean.len() - 40..])
}

/// Pad address to 32 bytes (ABI encoding)
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad uint256 to 32 bytes
pub fn pad_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Pad bytes32
pub fn pad_bytes32(val: &str) -> String {
    let clean = val.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Serialize u128 as string for JSON output
pub fn serialize_u128_as_string<S>(val: &u128, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&val.to_string())
}

/// Get ERC-20 decimals
pub async fn get_decimals(token: &str, rpc_url: &str) -> Result<u8> {
    // decimals() selector = 0x313ce567
    let data = "0x313ce567";
    let result = eth_call(token, data, rpc_url).await?;
    Ok(decode_u64_hex(&result) as u8)
}

/// Get ERC-20 balance
pub async fn get_balance_of(token: &str, owner: &str, rpc_url: &str) -> Result<u128> {
    // balanceOf(address) selector = 0x70a08231
    let data = format!("0x70a08231{}", pad_address(owner));
    let result = eth_call(token, &data, rpc_url).await?;
    Ok(decode_u256_hex(&result))
}

/// Get ERC-20 allowance
pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> Result<u128> {
    // allowance(address,address) selector = 0xdd62ed3e
    let data = format!(
        "0xdd62ed3e{}{}",
        pad_address(owner),
        pad_address(spender)
    );
    let result = eth_call(token, &data, rpc_url).await?;
    Ok(decode_u256_hex(&result))
}

/// Get ERC-20 total supply
pub async fn get_total_supply(token: &str, rpc_url: &str) -> Result<u128> {
    // totalSupply() selector = 0x18160ddd
    let result = eth_call(token, "0x18160ddd", rpc_url).await?;
    Ok(decode_u256_hex(&result))
}

/// Vault.getPool(bytes32) → (address pool, uint8 specialization)
pub async fn get_pool(pool_id: &str, vault: &str, rpc_url: &str) -> Result<(String, u8)> {
    // getPool(bytes32) selector = 0xf6c00927
    let data = format!("0xf6c00927{}", pad_bytes32(pool_id));
    let result = eth_call(vault, &data, rpc_url).await?;
    let clean = result.trim_start_matches("0x");
    if clean.len() < 128 {
        anyhow::bail!("getPool returned too short result");
    }
    let pool_addr = decode_address(&clean[0..64]);
    let specialization = decode_u64_hex(&clean[64..128]) as u8;
    Ok((pool_addr, specialization))
}

/// Vault.getPoolTokens(bytes32) → (tokens[], balances[], lastChangeBlock)
pub async fn get_pool_tokens(
    pool_id: &str,
    vault: &str,
    rpc_url: &str,
) -> Result<(Vec<String>, Vec<u128>, u64)> {
    // getPoolTokens(bytes32) selector = 0xf94d4668
    let data = format!("0xf94d4668{}", pad_bytes32(pool_id));
    let result = eth_call(vault, &data, rpc_url).await?;
    let hex = result.trim_start_matches("0x");

    // ABI decode: (address[], uint256[], uint256)
    // Structure: offset_tokens, offset_balances, lastChangeBlock, [tokens_array], [balances_array]
    if hex.len() < 192 {
        anyhow::bail!("getPoolTokens result too short");
    }

    let offset_tokens = (usize::from_str_radix(&hex[0..64], 16).unwrap_or(0)) * 2;
    let offset_balances = (usize::from_str_radix(&hex[64..128], 16).unwrap_or(0)) * 2;
    let last_change_block = decode_u64_hex(&hex[128..192]);

    // Decode tokens array
    let num_tokens = usize::from_str_radix(&hex[offset_tokens..offset_tokens + 64], 16).unwrap_or(0);
    let mut tokens = Vec::new();
    for i in 0..num_tokens {
        let start = offset_tokens + 64 + i * 64;
        let addr = decode_address(&hex[start..start + 64]);
        tokens.push(addr);
    }

    // Decode balances array
    let num_balances = usize::from_str_radix(&hex[offset_balances..offset_balances + 64], 16).unwrap_or(0);
    let mut balances = Vec::new();
    for i in 0..num_balances {
        let start = offset_balances + 64 + i * 64;
        let bal = decode_u256_hex(&hex[start..start + 64]);
        balances.push(bal);
    }

    Ok((tokens, balances, last_change_block))
}

/// Pool.getSwapFeePercentage() → uint256 (1e18 = 100%)
pub async fn get_swap_fee(pool_addr: &str, rpc_url: &str) -> Result<u128> {
    // getSwapFeePercentage() selector = 0x55c67628
    let result = eth_call(pool_addr, "0x55c67628", rpc_url).await?;
    Ok(decode_u256_hex(&result))
}

/// Pool.getNormalizedWeights() → uint256[] (1e18 = 100%)
pub async fn get_normalized_weights(pool_addr: &str, rpc_url: &str) -> Result<Vec<u128>> {
    // getNormalizedWeights() selector = 0xf89f27ed
    let result = eth_call(pool_addr, "0xf89f27ed", rpc_url).await?;
    let hex = result.trim_start_matches("0x");
    if hex.len() < 64 {
        return Ok(vec![]);
    }
    // ABI decode: uint256[]
    let offset = (usize::from_str_radix(&hex[0..64], 16).unwrap_or(0)) * 2;
    if offset + 64 > hex.len() {
        return Ok(vec![]);
    }
    let num = usize::from_str_radix(&hex[offset..offset + 64], 16).unwrap_or(0);
    let mut weights = Vec::new();
    for i in 0..num {
        let start = offset + 64 + i * 64;
        if start + 64 > hex.len() {
            break;
        }
        let w = decode_u256_hex(&hex[start..start + 64]);
        weights.push(w);
    }
    Ok(weights)
}

/// BalancerQueries.querySwap for GIVEN_IN
/// Returns amountOut
pub async fn query_swap(
    queries_contract: &str,
    pool_id: &str,
    asset_in: &str,
    asset_out: &str,
    amount_in: u128,
    rpc_url: &str,
) -> Result<u128> {
    // querySwap((bytes32,uint8,address,address,uint256,bytes),(address,bool,address,bool))
    // selector = 0xe969f6b3
    //
    // ABI encode: (SingleSwap, FundManagement)
    // SingleSwap: bytes32 poolId, uint8 kind, address assetIn, address assetOut, uint256 amount, bytes userData
    // FundManagement: address sender, bool fromInternalBalance, address recipient, bool toInternalBalance
    //
    // Using manual ABI encoding (structs with dynamic bytes require offset tracking)
    //
    // The struct layout (with dynamic bytes userData):
    // SingleSwap tuple offset from start of args = 0x40 (2 slots: singleSwap offset + funds tuple)
    // Actually for eth_call the whole thing is ABI-encoded args
    //
    // Let's build the calldata manually:
    // Function selector: 0xe969f6b3
    // arg[0] = offset to singleSwap tuple = 0x40 (64 bytes)
    // arg[1] = offset to funds tuple = ... (after singleSwap)
    //
    // SingleSwap tuple (with dynamic bytes):
    //   - bytes32 poolId
    //   - uint8 kind (GIVEN_IN = 0)
    //   - address assetIn
    //   - address assetOut
    //   - uint256 amount
    //   - bytes userData (offset within tuple, then length=0 + no data)
    //
    // FundManagement tuple (all static):
    //   - address sender (zero)
    //   - bool fromInternalBalance = false
    //   - address recipient (zero)
    //   - bool toInternalBalance = false

    let pool_id_clean = pool_id.trim_start_matches("0x");
    let asset_in_clean = asset_in.trim_start_matches("0x");
    let asset_out_clean = asset_out.trim_start_matches("0x");

    // ABI encoding for querySwap(SingleSwap, FundManagement):
    // - SingleSwap has dynamic member (bytes userData) → treated as dynamic tuple → offset pointer
    // - FundManagement has only static members → treated as static tuple → inlined
    //
    // Top-level head layout:
    //   [0..32]    pointer to SingleSwap data
    //   [32..160]  FundManagement inlined (4 * 32 = 128 bytes)
    // Total head = 32 + 128 = 160 = 0xa0
    //
    // SingleSwap data starts at offset 0xa0:
    //   poolId(32), kind(32), assetIn(32), assetOut(32), amount(32), bytes_offset(32) = head 6*32 = 192
    //   bytes_offset within tuple = 6 * 32 = 0xc0 (points past the 6 head slots)
    //   userData length = 0 (32 bytes)
    //   Total SingleSwap = 7 * 32 = 224 bytes

    let singleswap_offset = format!("{:064x}", 0xa0u64); // = 160 = head size

    // FundManagement inlined (4 * 32 bytes)
    let funds_inline = format!(
        "{}{}{}{}",
        pad_address("0x0000000000000000000000000000000000000000"), // sender = zero
        format!("{:064x}", 0u64),                                   // fromInternalBalance = false
        pad_address("0x0000000000000000000000000000000000000000"), // recipient = zero
        format!("{:064x}", 0u64),                                   // toInternalBalance = false
    );

    // SingleSwap data encoding
    // userData bytes offset within this tuple = 6 * 32 = 0xc0
    let singleswap_data = format!(
        "{:0>64}{:064x}{:0>64}{:0>64}{}{:064x}{:064x}",
        pool_id_clean,               // poolId (bytes32)
        0u8,                         // kind = GIVEN_IN (0)
        asset_in_clean,              // assetIn
        asset_out_clean,             // assetOut
        pad_u256(amount_in),         // amount
        6u64 * 32u64,                // offset to userData within this tuple = 0xc0
        0u64,                        // userData length = 0
    );

    let calldata = format!(
        "0xe969f6b3{}{}{}",
        singleswap_offset,
        funds_inline,
        singleswap_data,
    );

    let result = eth_call(queries_contract, &calldata, rpc_url).await?;
    Ok(decode_u256_hex(&result))
}
