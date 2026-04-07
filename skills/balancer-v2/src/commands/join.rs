/// join command — add liquidity to a Balancer V2 pool via Vault.joinPool()

use anyhow::Result;
use serde::Serialize;
use tokio::time::{sleep, Duration};

use crate::config;
use crate::onchainos;
use crate::rpc;

#[derive(Debug, Serialize)]
struct JoinResult {
    tx_hash: String,
    pool_id: String,
    chain_id: u64,
    dry_run: bool,
    tokens: Vec<String>,
    amounts_in: Vec<String>,
}

pub async fn run(
    pool_id: &str,
    amounts: &[f64], // amounts per token (use 0.0 to skip that token)
    chain_id: u64,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::rpc_url(chain_id);
    let vault = config::VAULT_ADDRESS;

    // getPoolTokens to get ordered token list
    let (tokens, _balances, _) = rpc::get_pool_tokens(pool_id, vault, rpc_url).await?;

    if tokens.is_empty() {
        anyhow::bail!("No tokens found for pool {}", pool_id);
    }

    if amounts.len() != tokens.len() {
        anyhow::bail!(
            "Pool has {} tokens but {} amounts provided",
            tokens.len(),
            amounts.len()
        );
    }

    // Get decimals and compute raw amounts
    let mut raw_amounts: Vec<u128> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        let decimals = rpc::get_decimals(token, rpc_url).await.unwrap_or(18);
        let raw = (amounts[i] * 10f64.powi(decimals as i32)) as u128;
        raw_amounts.push(raw);
    }

    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    // Build joinPool calldata
    // joinPool(bytes32,address,address,(address[],uint256[],bytes,bool))
    // selector: 0xb95cac28
    //
    // userData for EXACT_TOKENS_IN_FOR_BPT_OUT (kind=1):
    //   abi.encode(uint256(1), uint256[] amountsIn, uint256 minimumBPT=0)
    let user_data = build_join_user_data(&raw_amounts);
    let calldata = build_join_calldata(pool_id, &wallet, &wallet, &tokens, &raw_amounts, &user_data);

    // Approve each token (if not dry run)
    if !dry_run {
        for (i, token) in tokens.iter().enumerate() {
            if raw_amounts[i] > 0 {
                let allowance = rpc::get_allowance(token, &wallet, vault, rpc_url).await?;
                if allowance < raw_amounts[i] {
                    eprintln!("Approving token {}...", token);
                    // Ask user to confirm before executing join (documented in SKILL.md)
                    let approve_result =
                        onchainos::erc20_approve(chain_id, token, vault, u128::MAX, None, false).await?;
                    let approve_hash = onchainos::extract_tx_hash(&approve_result);
                    eprintln!("Approve tx: {}", approve_hash);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        vault,
        &calldata,
        None,
        None,
        dry_run,
        true, // --force for DEX ops
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let output = JoinResult {
        tx_hash,
        pool_id: pool_id.to_string(),
        chain_id,
        dry_run,
        tokens,
        amounts_in: raw_amounts.iter().map(|a| a.to_string()).collect(),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Build userData for EXACT_TOKENS_IN_FOR_BPT_OUT join
/// abi.encode(uint256 kind=1, uint256[] amountsIn, uint256 minimumBPT=0)
fn build_join_user_data(amounts: &[u128]) -> Vec<u8> {
    let mut data: Vec<u8> = Vec::new();

    // kind = 1 (EXACT_TOKENS_IN_FOR_BPT_OUT)
    let mut kind_bytes = [0u8; 32];
    kind_bytes[31] = 1;
    data.extend_from_slice(&kind_bytes);

    // offset to amountsIn array = 3 * 32 = 96 (after kind, offset, minimumBPT)
    // Actually abi.encode(uint256, uint256[], uint256):
    // [0..32]: kind = 1
    // [32..64]: offset to array = 96 (3 * 32)
    // [64..96]: minimumBPT = 0
    // [96..128]: array length
    // [128..]: array elements

    // offset to array
    let offset: u64 = 96;
    let mut offset_bytes = [0u8; 32];
    offset_bytes[24..].copy_from_slice(&offset.to_be_bytes());
    data.extend_from_slice(&offset_bytes);

    // minimumBPT = 0
    data.extend_from_slice(&[0u8; 32]);

    // array length
    let len = amounts.len() as u64;
    let mut len_bytes = [0u8; 32];
    len_bytes[24..].copy_from_slice(&len.to_be_bytes());
    data.extend_from_slice(&len_bytes);

    // array elements
    for &a in amounts {
        let mut elem = [0u8; 32];
        let bytes = a.to_be_bytes();
        elem[16..].copy_from_slice(&bytes);
        data.extend_from_slice(&elem);
    }

    data
}

/// Build calldata for Vault.joinPool()
/// joinPool(bytes32,address,address,(address[],uint256[],bytes,bool))
/// selector: 0xb95cac28
fn build_join_calldata(
    pool_id: &str,
    sender: &str,
    recipient: &str,
    tokens: &[String],
    max_amounts_in: &[u128],
    user_data: &[u8],
) -> String {
    // ABI encode:
    // arg0: bytes32 poolId (static, 32 bytes)
    // arg1: address sender (static, 32 bytes)
    // arg2: address recipient (static, 32 bytes)
    // arg3: offset to JoinPoolRequest tuple (dynamic tuple)
    //
    // JoinPoolRequest tuple:
    //   assets: address[] (dynamic)
    //   maxAmountsIn: uint256[] (dynamic)
    //   userData: bytes (dynamic)
    //   fromInternalBalance: bool (static)
    //
    // Since JoinPoolRequest has dynamic members, arg3 is an offset pointer
    //
    // Head area: arg0(32) + arg1(32) + arg2(32) + arg3_offset(32) = 128 = 0x80
    // JoinPoolRequest starts at 0x80
    //
    // JoinPoolRequest head:
    //   assets_offset(32), maxAmountsIn_offset(32), userData_offset(32), fromInternalBalance(32) = 128 bytes
    //
    // assets_offset = 128 (4 * 32 = 0x80, relative to start of JoinPoolRequest)
    // maxAmountsIn_offset = 128 + 32 + tokens.len() * 32 (after assets array)
    // userData_offset = maxAmountsIn_offset's position + 32 + tokens.len() * 32
    //
    // Let's compute:
    let n = tokens.len();
    let assets_offset: usize = 4 * 32; // 128 — offset within JoinPoolRequest to assets array
    let assets_array_size: usize = (1 + n) * 32; // length word + n elements
    let max_amounts_offset: usize = assets_offset + assets_array_size;
    let max_amounts_array_size: usize = (1 + n) * 32;
    let user_data_offset: usize = max_amounts_offset + max_amounts_array_size;

    // user_data length + data (padded to 32 bytes)
    let user_data_len = user_data.len();
    let user_data_padded_len = ((user_data_len + 31) / 32) * 32;

    // Top-level offset to JoinPoolRequest = 4 * 32 = 128 = 0x80
    let request_offset: usize = 4 * 32;

    let mut hex = String::from("0xb95cac28");

    // arg0: poolId
    hex.push_str(&format!("{:0>64}", pool_id.trim_start_matches("0x")));
    // arg1: sender
    hex.push_str(&format!("{:0>64}", sender.trim_start_matches("0x")));
    // arg2: recipient
    hex.push_str(&format!("{:0>64}", recipient.trim_start_matches("0x")));
    // arg3: offset to JoinPoolRequest
    hex.push_str(&format!("{:064x}", request_offset));

    // JoinPoolRequest head
    hex.push_str(&format!("{:064x}", assets_offset));
    hex.push_str(&format!("{:064x}", max_amounts_offset));
    hex.push_str(&format!("{:064x}", user_data_offset));
    hex.push_str(&format!("{:064x}", 0u64)); // fromInternalBalance = false

    // assets array
    hex.push_str(&format!("{:064x}", n));
    for token in tokens {
        hex.push_str(&format!("{:0>64}", token.trim_start_matches("0x")));
    }

    // maxAmountsIn array
    hex.push_str(&format!("{:064x}", n));
    for &amt in max_amounts_in {
        hex.push_str(&format!("{:064x}", amt));
    }

    // userData
    hex.push_str(&format!("{:064x}", user_data_len));
    for chunk in user_data.chunks(32) {
        let mut padded = [0u8; 32];
        padded[..chunk.len()].copy_from_slice(chunk);
        hex.push_str(&hex::encode(padded));
    }
    // If user_data is empty, no data bytes needed (just the length = 0)
    if user_data_padded_len > user_data_len {
        // Padding already handled by chunks(32) since last chunk gets zero-padded
    }

    hex
}
