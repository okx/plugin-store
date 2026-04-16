use anyhow::Context;
use serde_json::json;

fn serialize_u128_as_string<S: serde::Serializer>(v: &u128, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&v.to_string())
}

/// Low-level eth_call via JSON-RPC. Returns the raw hex result string (may include "0x" prefix).
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call JSON parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }

    let result = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_call: missing result field in response"))?;
    Ok(result.to_string())
}

/// Decode a 32-byte ABI word as u128 (e.g. for balances).
pub fn decode_u128(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    u128::from_str_radix(&clean[clean.len().saturating_sub(32)..], 16).unwrap_or(0)
}

/// Decode a 32-byte ABI word as u64.
pub fn decode_u64(hex: &str) -> u64 {
    let clean = hex.trim_start_matches("0x");
    u64::from_str_radix(&clean[clean.len().saturating_sub(16)..], 16).unwrap_or(0)
}

/// Decode a 32-byte ABI int24/int32 tick value (sign-extended).
pub fn decode_tick(hex_str: &str) -> i32 {
    let clean = hex_str.trim_start_matches("0x");
    let last8 = &clean[clean.len().saturating_sub(8)..];
    u32::from_str_radix(last8, 16).unwrap_or(0) as i32
}

/// Decode an ABI-encoded address from a 32-byte word (last 20 bytes / 40 hex chars).
pub fn decode_address(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        format!("0x{:0>40}", clean)
    }
}

/// Pad an address to a 32-byte ABI word (left-pad with zeros).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u256 from a big integer (given as decimal string).
pub fn pad_u256_dec(value: u64) -> String {
    format!("{:064x}", value)
}

/// Query ERC-721 balanceOf(owner).
pub async fn nft_balance_of(
    nft_contract: &str,
    owner: &str,
    rpc_url: &str,
) -> anyhow::Result<u64> {
    // balanceOf(address) selector = 0x70a08231
    let calldata = format!("0x70a08231{}", pad_address(owner));
    let result = eth_call(nft_contract, &calldata, rpc_url).await?;
    Ok(decode_u64(&result))
}

/// Query ERC-721 tokenOfOwnerByIndex(owner, index).
pub async fn token_of_owner_by_index(
    nft_contract: &str,
    owner: &str,
    index: u64,
    rpc_url: &str,
) -> anyhow::Result<u64> {
    // tokenOfOwnerByIndex(address,uint256) selector = 0x2f745c59
    let calldata = format!(
        "0x2f745c59{}{}",
        pad_address(owner),
        pad_u256_dec(index)
    );
    let result = eth_call(nft_contract, &calldata, rpc_url).await?;
    Ok(decode_u64(&result))
}

/// Query NonfungiblePositionManager.positions(tokenId).
/// Returns raw ABI response (multiple fields).
pub async fn get_position(
    nft_contract: &str,
    token_id: u64,
    rpc_url: &str,
) -> anyhow::Result<PositionData> {
    // positions(uint256) selector = 0x99fbab88
    let calldata = format!("0x99fbab88{}", pad_u256_dec(token_id));
    let result = eth_call(nft_contract, &calldata, rpc_url).await?;
    parse_position_data(&result, token_id)
}

#[derive(Debug, serde::Serialize)]
pub struct PositionData {
    pub token_id: u64,
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub liquidity: u128,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub tokens_owed0: u128,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub tokens_owed1: u128,
}

fn parse_position_data(hex: &str, token_id: u64) -> anyhow::Result<PositionData> {
    let clean = hex.trim_start_matches("0x");
    // ABI response layout (each field is 32 bytes / 64 hex chars):
    // [0]  nonce (uint96)
    // [1]  operator (address)
    // [2]  token0 (address)
    // [3]  token1 (address)
    // [4]  fee (uint24)
    // [5]  tickLower (int24)
    // [6]  tickUpper (int24)
    // [7]  liquidity (uint128)
    // [8]  feeGrowthInside0LastX128 (uint256)
    // [9]  feeGrowthInside1LastX128 (uint256)
    // [10] tokensOwed0 (uint128)
    // [11] tokensOwed1 (uint128)
    if clean.len() < 64 * 12 {
        anyhow::bail!("positions() response too short (token_id={})", token_id);
    }
    let word = |i: usize| &clean[i * 64..(i + 1) * 64];

    let token0 = decode_address(word(2));
    let token1 = decode_address(word(3));
    let fee = u32::from_str_radix(&word(4)[56..], 16).unwrap_or(0);
    let tick_lower = decode_tick(word(5));
    let tick_upper = decode_tick(word(6));
    let liquidity = decode_u128(word(7));
    let tokens_owed0 = decode_u128(word(10));
    let tokens_owed1 = decode_u128(word(11));

    Ok(PositionData {
        token_id,
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

/// Query NonfungiblePositionManager.ownerOf(tokenId).
pub async fn owner_of(
    nft_contract: &str,
    token_id: u64,
    rpc_url: &str,
) -> anyhow::Result<String> {
    // ownerOf(uint256) selector = 0x6352211e
    let calldata = format!("0x6352211e{}", pad_u256_dec(token_id));
    let result = eth_call(nft_contract, &calldata, rpc_url).await?;
    Ok(decode_address(&result))
}

/// Format a wei-denominated CAKE amount to 6 decimal places using integer arithmetic.
/// Avoids f64 precision loss (which starts above ~9,007 CAKE with `as f64 / 1e18`).
pub fn format_cake_wei(wei: u128) -> String {
    let whole = wei / 1_000_000_000_000_000_000u128;
    let frac  = (wei % 1_000_000_000_000_000_000u128) / 1_000_000_000_000u128; // 6 dp
    format!("{}.{:06}", whole, frac)
}

/// Query MasterChefV3.pendingCake(tokenId).
pub async fn pending_cake(
    masterchef: &str,
    token_id: u64,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    // pendingCake(uint256) selector = 0xce5f39c6
    let calldata = format!("0xce5f39c6{}", pad_u256_dec(token_id));
    let result = eth_call(masterchef, &calldata, rpc_url).await?;
    Ok(decode_u128(&result))
}

#[derive(Debug, serde::Serialize)]
pub struct UserPositionInfo {
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub liquidity: u128,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub boost_liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub reward: u128,
    pub user: String,
    pub pid: u64,
}

/// Query MasterChefV3.userPositionInfos(tokenId).
pub async fn user_position_infos(
    masterchef: &str,
    token_id: u64,
    rpc_url: &str,
) -> anyhow::Result<UserPositionInfo> {
    // userPositionInfos(uint256) selector = 0x3b1acf74
    let calldata = format!("0x3b1acf74{}", pad_u256_dec(token_id));
    let result = eth_call(masterchef, &calldata, rpc_url).await?;
    parse_user_position_info(&result)
}

fn parse_user_position_info(hex: &str) -> anyhow::Result<UserPositionInfo> {
    let clean = hex.trim_start_matches("0x");
    // userPositionInfos returns:
    // [0] liquidity (uint128)
    // [1] boostLiquidity (uint128)
    // [2] tickLower (int24)
    // [3] tickUpper (int24)
    // [4] rewardGrowthInside (uint256)
    // [5] reward (uint128)
    // [6] user (address)
    // [7] pid (uint256)
    // [8] boostMultiplier (uint256)
    if clean.len() < 64 * 9 {
        anyhow::bail!("userPositionInfos() response too short");
    }
    let word = |i: usize| &clean[i * 64..(i + 1) * 64];

    Ok(UserPositionInfo {
        liquidity: decode_u128(word(0)),
        boost_liquidity: decode_u128(word(1)),
        tick_lower: decode_tick(word(2)),
        tick_upper: decode_tick(word(3)),
        reward: decode_u128(word(5)),
        user: decode_address(word(6)),
        pid: decode_u64(word(7)),
    })
}

#[derive(Debug, serde::Serialize)]
pub struct PoolInfo {
    pub pid: u64,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub alloc_point: u128,
    pub v3_pool: String,
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub total_liquidity: u128,
    #[serde(serialize_with = "serialize_u128_as_string")]
    pub total_boost_liquidity: u128,
}

/// Query MasterChefV3.poolLength().
pub async fn pool_length(masterchef: &str, rpc_url: &str) -> anyhow::Result<u64> {
    // poolLength() selector = 0x081e3eda
    let result = eth_call(masterchef, "0x081e3eda", rpc_url).await?;
    Ok(decode_u64(&result))
}

/// Query MasterChefV3.poolInfo(pid).
pub async fn pool_info(
    masterchef: &str,
    pid: u64,
    rpc_url: &str,
) -> anyhow::Result<PoolInfo> {
    // poolInfo(uint256) selector = 0x1526fe27
    let calldata = format!("0x1526fe27{}", pad_u256_dec(pid));
    let result = eth_call(masterchef, &calldata, rpc_url).await?;
    parse_pool_info(&result, pid)
}

/// Scan ERC-721 Transfer events to discover token IDs currently staked by `wallet` in `masterchef`.
///
/// Strategy:
///   1. Query Transfer(wallet → masterchef) logs — all NFTs ever deposited for farming.
///   2. Query Transfer(masterchef → wallet) logs — all NFTs ever withdrawn/unfarmed.
///   3. Candidates = deposited_set − withdrawn_set (net staked by log history).
///   4. For each candidate, call `userPositionInfos(tokenId)` and keep only those where
///      `info.user == wallet` — this is the authoritative on-chain confirmation.
///
/// Returns `(token_ids, discovery_note)`.
/// On RPC log query failure, returns `(vec![], warning_message)` so the caller can surface it.
pub async fn scan_staked_token_ids(
    nft_contract: &str,
    masterchef: &str,
    wallet: &str,
    from_block: u64,
    rpc_url: &str,
) -> (Vec<u64>, String) {
    // ERC-721 Transfer(address indexed from, address indexed to, uint256 indexed tokenId)
    const TRANSFER_SIG: &str =
        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

    let wallet_lower = wallet.trim_start_matches("0x").to_lowercase();
    let masterchef_lower = masterchef.trim_start_matches("0x").to_lowercase();
    let wallet_padded = format!("0x{:0>64}", wallet_lower);
    let masterchef_padded = format!("0x{:0>64}", masterchef_lower);

    let from_block_hex = format!("0x{:x}", from_block);

    // Deposits: wallet → masterchef
    let deposited = match get_transfer_logs(
        nft_contract,
        TRANSFER_SIG,
        &wallet_padded,
        &masterchef_padded,
        &from_block_hex,
        rpc_url,
    )
    .await
    {
        Ok(ids) => ids,
        Err(e) => {
            return (
                vec![],
                format!(
                    "Staked position auto-discovery unavailable: eth_getLogs failed ({}). \
                     Use --include-staked <token_ids> to view staked positions manually.",
                    e
                ),
            );
        }
    };

    if deposited.is_empty() {
        return (vec![], "No historical stake deposits found for this wallet.".to_string());
    }

    // Withdrawals: masterchef → wallet
    let withdrawn = match get_transfer_logs(
        nft_contract,
        TRANSFER_SIG,
        &masterchef_padded,
        &wallet_padded,
        &from_block_hex,
        rpc_url,
    )
    .await
    {
        Ok(ids) => ids,
        Err(_) => vec![], // If we can't get withdrawals, conservatively keep all deposits as candidates
    };

    let withdrawn_set: std::collections::HashSet<u64> = withdrawn.into_iter().collect();
    let candidates: Vec<u64> = {
        let mut seen = std::collections::HashSet::new();
        deposited
            .into_iter()
            .filter(|id| !withdrawn_set.contains(id) && seen.insert(*id))
            .collect()
    };

    let note = format!(
        "Auto-discovered {} candidate token ID(s) from Transfer logs; verifying on-chain.",
        candidates.len()
    );
    (candidates, note)
}

async fn get_transfer_logs(
    contract: &str,
    event_sig: &str,
    topic1: &str,
    topic2: &str,
    from_block: &str,
    rpc_url: &str,
) -> anyhow::Result<Vec<u64>> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_getLogs",
        "params": [{
            "address": contract,
            "topics": [event_sig, topic1, topic2, null],
            "fromBlock": from_block,
            "toBlock": "latest"
        }],
        "id": 1
    });

    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("eth_getLogs HTTP request failed")?
        .json()
        .await
        .context("eth_getLogs JSON parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("{}", err);
    }

    let logs = resp["result"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("eth_getLogs: missing result array"))?;

    // topic[3] = indexed tokenId (all 3 Transfer params are indexed in ERC-721)
    let token_ids = logs
        .iter()
        .filter_map(|log| {
            log["topics"]
                .as_array()?
                .get(3)?
                .as_str()
                .map(decode_u64)
        })
        .collect();

    Ok(token_ids)
}

fn parse_pool_info(hex: &str, pid: u64) -> anyhow::Result<PoolInfo> {
    let clean = hex.trim_start_matches("0x");
    // poolInfo returns:
    // [0] allocPoint (uint256)
    // [1] v3Pool (address)
    // [2] token0 (address)
    // [3] token1 (address)
    // [4] fee (uint24)
    // [5] totalLiquidity (uint256)
    // [6] totalBoostLiquidity (uint256)
    if clean.len() < 64 * 7 {
        anyhow::bail!("poolInfo() response too short for pid={}", pid);
    }
    let word = |i: usize| &clean[i * 64..(i + 1) * 64];

    Ok(PoolInfo {
        pid,
        alloc_point: decode_u128(word(0)),
        v3_pool: decode_address(word(1)),
        token0: decode_address(word(2)),
        token1: decode_address(word(3)),
        fee: u32::from_str_radix(&word(4)[56..], 16).unwrap_or(0),
        total_liquidity: decode_u128(word(5)),
        total_boost_liquidity: decode_u128(word(6)),
    })
}
