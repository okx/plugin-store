use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use reqwest::Client;
use serde_json::{json, Value};

const SOLANA_RPC: &str = "https://api.mainnet-beta.solana.com";
const SOLANA_RPC_FALLBACK: &str = "https://rpc.ankr.com/solana";

/// POST to Solana RPC.
/// Retries primary endpoint up to 5 times with increasing backoff before falling
/// back to the secondary. The secondary (ankr) may have stale data, so we prefer
/// to wait for the primary rather than accepting potentially wrong results.
async fn rpc_call(client: &Client, body: &Value) -> anyhow::Result<Value> {
    // Primary: up to 5 attempts with exponential backoff
    for attempt in 0u32..5 {
        if attempt > 0 {
            let delay = 300 * (1u64 << attempt.min(4)); // 600, 1200, 2400, 2400 ms
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }
        let resp = client.post(SOLANA_RPC).json(body).send().await?;
        if resp.status().as_u16() == 429 {
            continue;
        }
        return resp.json::<Value>().await.map_err(|e| anyhow::anyhow!("RPC JSON parse: {e}"));
    }
    // Last resort: secondary endpoint (may have stale account data)
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let resp = client.post(SOLANA_RPC_FALLBACK).json(body).send().await?;
    resp.json::<Value>().await.map_err(|e| anyhow::anyhow!("RPC JSON parse (fallback): {e}"))
}

pub async fn get_account_data(client: &Client, address: &str) -> anyhow::Result<Vec<u8>> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [address, {"encoding": "base64"}]
    });
    let resp: Value = rpc_call(client, &body).await?;

    let data_b64 = resp["result"]["value"]["data"][0]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Account not found: {address}"))?;

    Ok(B64.decode(data_b64)?)
}

/// Find the user's token account for a given mint.
/// Prefers the ATA if it exists; falls back to any token account via getTokenAccountsByOwner.
/// Returns (account_pubkey, exists).
pub async fn find_token_account(
    client: &Client,
    wallet: &str,
    mint: &str,
    ata: &str, // precomputed ATA address to try first
) -> anyhow::Result<(String, bool)> {
    // Fast path: check if ATA exists
    if account_exists(client, ata).await? {
        return Ok((ata.to_string(), true));
    }

    // Slow path: find any token account for this mint
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet,
            {"mint": mint},
            {"encoding": "jsonParsed"}
        ]
    });
    let resp: Value = rpc_call(client, &body).await?;

    if let Some(acc) = resp["result"]["value"].as_array().and_then(|a| a.first()) {
        let pubkey = acc["pubkey"].as_str().unwrap_or("").to_string();
        if !pubkey.is_empty() {
            return Ok((pubkey, true));
        }
    }

    Ok((ata.to_string(), false))
}

pub async fn account_exists(client: &Client, address: &str) -> anyhow::Result<bool> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [address, {"encoding": "base64"}]
    });
    let resp: Value = rpc_call(client, &body).await?;
    // If the RPC returned an error object, propagate it — don't silently return false.
    if let Some(err) = resp.get("error") {
        anyhow::bail!("RPC error checking account {address}: {err}");
    }
    if resp["result"]["value"].is_object() {
        return Ok(true);
    }
    // Primary returned null (may be stale/rate-limited). Cross-check with fallback.
    // If fallback confirms existence, trust it to avoid spurious init instructions.
    if let Ok(resp2) = client.post(SOLANA_RPC_FALLBACK).json(&body).send().await {
        if let Ok(v) = resp2.json::<Value>().await {
            if v["result"]["value"].is_object() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub async fn get_latest_blockhash(client: &Client) -> anyhow::Result<[u8; 32]> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "confirmed"}]
    });
    let resp: Value = rpc_call(client, &body).await?;

    let hash_str = resp["result"]["value"]["blockhash"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No blockhash in RPC response"))?;

    let bytes = bs58::decode(hash_str).into_vec()?;
    anyhow::ensure!(bytes.len() == 32, "Invalid blockhash length: {}", bytes.len());
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Parsed fields from an LbPair account
pub struct LbPairInfo {
    pub active_id: i32,
    pub bin_step: u16,
    pub token_x_mint: [u8; 32],
    pub token_y_mint: [u8; 32],
    pub reserve_x: [u8; 32],
    pub reserve_y: [u8; 32],
}

/// Parse an LbPair account buffer.
///
/// Offsets verified against Meteora DLMM IDL struct layout:
///   8  anchor discriminator
///   +68 fields before active_id  → offset 76
///   active_id   i32  [76..80]
///   bin_step    u16  [80..82]
///   6 bytes pad  →  offset 88
///   token_x_mint Pubkey [88..120]
///   token_y_mint Pubkey [120..152]
///   reserve_x    Pubkey [152..184]
///   reserve_y    Pubkey [184..216]
pub fn parse_lb_pair(data: &[u8]) -> anyhow::Result<LbPairInfo> {
    anyhow::ensure!(
        data.len() >= 216,
        "LbPair account data too short: {} bytes (expected ≥216)",
        data.len()
    );

    let active_id = i32::from_le_bytes(data[76..80].try_into()?);
    let bin_step = u16::from_le_bytes(data[80..82].try_into()?);

    let mut token_x_mint = [0u8; 32];
    token_x_mint.copy_from_slice(&data[88..120]);
    let mut token_y_mint = [0u8; 32];
    token_y_mint.copy_from_slice(&data[120..152]);
    let mut reserve_x = [0u8; 32];
    reserve_x.copy_from_slice(&data[152..184]);
    let mut reserve_y = [0u8; 32];
    reserve_y.copy_from_slice(&data[184..216]);

    Ok(LbPairInfo {
        active_id,
        bin_step,
        token_x_mint,
        token_y_mint,
        reserve_x,
        reserve_y,
    })
}

/// Parse the 70 liquidity shares from a DLMM PositionV2 account.
/// Shares are stored at [72..1192] as 70 × u128 LE.
/// shares[i] corresponds to the bin at (lower_bin_id + i) within the position.
pub fn parse_position_shares(data: &[u8]) -> [u128; 70] {
    let mut shares = [0u128; 70];
    if data.len() < 1192 {
        return shares;
    }
    for (i, chunk) in data[72..1192].chunks_exact(16).enumerate().take(70) {
        shares[i] = u128::from_le_bytes(chunk.try_into().unwrap_or([0u8; 16]));
    }
    shares
}

/// Parse (amount_x, amount_y, liquidity_supply) for one bin within a BinArray.
///
/// BinArray account layout (10136 bytes):
///   [0..8]   Anchor discriminator
///   [8..16]  index (i64 LE)
///   [16..24] version_info (u64)
///   [24..56] lb_pair (Pubkey, 32 bytes)
///   [56..]   bins: [Bin; 70], each Bin = 144 bytes
///
/// Bin struct (Meteora DLMM source, 144 bytes total):
///   [+0..+8]   amount_x     (u64 LE)
///   [+8..+16]  amount_y     (u64 LE)
///   [+16..+32] price        (u128 LE, Q64.64 fixed-point)
///   [+32..+48] liquidity_supply (u128 LE)
///   [+48..+144] reward/fee fields (not used here)
pub fn parse_bin_at(data: &[u8], pos_in_array: usize) -> (u64, u64, u128) {
    const HEADER: usize = 56;
    const BIN_SIZE: usize = 144;
    let base = HEADER + pos_in_array * BIN_SIZE;
    if data.len() < base + 48 {
        return (0, 0, 0);
    }
    let amount_x =
        u64::from_le_bytes(data[base..base + 8].try_into().unwrap_or([0u8; 8]));
    let amount_y =
        u64::from_le_bytes(data[base + 8..base + 16].try_into().unwrap_or([0u8; 8]));
    let liquidity_supply =
        u128::from_le_bytes(data[base + 32..base + 48].try_into().unwrap_or([0u8; 16]));
    (amount_x, amount_y, liquidity_supply)
}

/// Check whether a DLMM PositionV2 account has any non-zero liquidity shares.
///
/// Liquidity shares: 70 × u128 LE at offsets [72..1192].
/// All-zero means the position is empty — attempting remove_liquidity will error.
pub fn position_has_liquidity(data: &[u8]) -> bool {
    if data.len() < 1192 {
        return false;
    }
    data[72..1192].chunks_exact(16).any(|chunk| {
        u128::from_le_bytes(chunk.try_into().unwrap_or([0u8; 16])) != 0
    })
}

/// Parse lower_bin_id and upper_bin_id from a DLMM Position account (8120 bytes).
///
/// Offsets verified against on-chain data:
///   lower_bin_id  i32  [7912..7916]
///   upper_bin_id  i32  [7916..7920]
pub fn parse_position_bins(data: &[u8]) -> anyhow::Result<(i32, i32)> {
    anyhow::ensure!(
        data.len() >= 7920,
        "Position account data too short: {} bytes (expected ≥7920)",
        data.len()
    );
    let lower_bin_id = i32::from_le_bytes(data[7912..7916].try_into()?);
    let upper_bin_id = i32::from_le_bytes(data[7916..7920].try_into()?);
    Ok((lower_bin_id, upper_bin_id))
}

/// Get token decimals from SPL Mint account data.
/// Decimals live at offset 44 in the standard Mint layout.
pub fn parse_mint_decimals(data: &[u8]) -> u8 {
    if data.len() > 44 {
        data[44]
    } else {
        6
    }
}

/// On-chain DLMM position (minimal fields, sourced directly from RPC).
pub struct OnChainPosition {
    pub address: String,
    pub lb_pair: String,
    pub owner: String,
    pub lower_bin_id: i32,
    pub upper_bin_id: i32,
}

/// Scan on-chain DLMM PositionV2 accounts (8120 bytes) owned by a wallet.
///
/// Uses `getProgramAccounts` with:
///   - dataSize filter: 8120
///   - memcmp at offset 40: owner pubkey (bs58)
///   - optionally memcmp at offset 8: lb_pair pubkey (pool filter)
///
/// Layout offsets (Anchor PositionV2):
///   [0..8]    discriminator
///   [8..40]   lb_pair  (Pubkey)
///   [40..72]  owner    (Pubkey)
///   [7912..7916]  lower_bin_id (i32 LE)
///   [7916..7920]  upper_bin_id (i32 LE)
pub async fn get_dlmm_positions_by_owner(
    client: &Client,
    program_id: &str,
    owner_bs58: &str,
    pool_filter: Option<&str>,
) -> anyhow::Result<Vec<OnChainPosition>> {
    let mut filters = vec![
        serde_json::json!({"dataSize": 8120}),
        serde_json::json!({"memcmp": {"offset": 40, "bytes": owner_bs58}}),
    ];
    if let Some(pool) = pool_filter {
        filters.push(serde_json::json!({"memcmp": {"offset": 8, "bytes": pool}}));
    }

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getProgramAccounts",
        "params": [
            program_id,
            {
                "encoding": "base64",
                "filters": filters
            }
        ]
    });

    let resp: serde_json::Value = rpc_call(client, &body).await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("getProgramAccounts RPC error: {err}");
    }

    let accounts = resp["result"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("getProgramAccounts: unexpected response format"))?;

    let mut positions = Vec::new();
    for account in accounts {
        let pubkey = account["pubkey"].as_str().unwrap_or("").to_string();
        if pubkey.is_empty() {
            continue;
        }
        let data_b64 = match account["account"]["data"][0].as_str() {
            Some(s) => s,
            None => continue,
        };
        let data = match B64.decode(data_b64) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if data.len() < 7920 {
            continue;
        }

        let lb_pair = bs58::encode(&data[8..40]).into_string();
        let owner = bs58::encode(&data[40..72]).into_string();
        let (lower_bin_id, upper_bin_id) = parse_position_bins(&data)?;

        positions.push(OnChainPosition {
            address: pubkey,
            lb_pair,
            owner,
            lower_bin_id,
            upper_bin_id,
        });
    }

    Ok(positions)
}
