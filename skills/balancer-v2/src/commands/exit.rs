/// exit command — remove liquidity from a Balancer V2 pool via Vault.exitPool()

use anyhow::Result;
use serde::Serialize;

use crate::config;
use crate::onchainos;
use crate::rpc;

#[derive(Debug, Serialize)]
struct ExitResult {
    tx_hash: String,
    pool_id: String,
    bpt_amount_in: String,
    chain_id: u64,
    dry_run: bool,
}

pub async fn run(
    pool_id: &str,
    bpt_amount: f64,
    chain_id: u64,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::rpc_url(chain_id);
    let vault = config::VAULT_ADDRESS;

    // getPoolTokens
    let (tokens, _balances, _) = rpc::get_pool_tokens(pool_id, vault, rpc_url).await?;

    if tokens.is_empty() {
        anyhow::bail!("No tokens found for pool {}", pool_id);
    }

    let bpt_raw = (bpt_amount * 1e18) as u128;

    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    // userData for EXACT_BPT_IN_FOR_TOKENS_OUT (kind=1):
    // abi.encode(uint256(1), uint256 bptAmountIn)
    let user_data = build_exit_user_data(bpt_raw);

    // minAmountsOut = [0, 0, ...] for simplicity (accept whatever we get)
    let min_amounts_out = vec![0u128; tokens.len()];

    let calldata = build_exit_calldata(pool_id, &wallet, &wallet, &tokens, &min_amounts_out, &user_data);

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
    let output = ExitResult {
        tx_hash,
        pool_id: pool_id.to_string(),
        bpt_amount_in: bpt_raw.to_string(),
        chain_id,
        dry_run,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Build userData for EXACT_BPT_IN_FOR_TOKENS_OUT exit
/// abi.encode(uint256 kind=1, uint256 bptAmountIn)
fn build_exit_user_data(bpt_amount: u128) -> Vec<u8> {
    let mut data = Vec::new();
    // kind = 1
    let mut kind_bytes = [0u8; 32];
    kind_bytes[31] = 1;
    data.extend_from_slice(&kind_bytes);
    // bptAmountIn
    let mut amt_bytes = [0u8; 32];
    let bytes = bpt_amount.to_be_bytes();
    amt_bytes[16..].copy_from_slice(&bytes);
    data.extend_from_slice(&amt_bytes);
    data
}

/// Build calldata for Vault.exitPool()
/// exitPool(bytes32,address,address,(address[],uint256[],bytes,bool))
/// selector: 0x8bdb3913
fn build_exit_calldata(
    pool_id: &str,
    sender: &str,
    recipient: &str,
    tokens: &[String],
    min_amounts_out: &[u128],
    user_data: &[u8],
) -> String {
    // Same structure as joinPool but selector 0x8bdb3913 and ExitPoolRequest
    // ExitPoolRequest: assets[], minAmountsOut[], userData, toInternalBalance

    let n = tokens.len();
    let assets_offset: usize = 4 * 32;
    let assets_array_size: usize = (1 + n) * 32;
    let min_amounts_offset: usize = assets_offset + assets_array_size;
    let min_amounts_array_size: usize = (1 + n) * 32;
    let user_data_offset: usize = min_amounts_offset + min_amounts_array_size;
    let user_data_len = user_data.len();
    let request_offset: usize = 4 * 32;

    let mut hex = String::from("0x8bdb3913");

    hex.push_str(&format!("{:0>64}", pool_id.trim_start_matches("0x")));
    hex.push_str(&format!("{:0>64}", sender.trim_start_matches("0x")));
    hex.push_str(&format!("{:0>64}", recipient.trim_start_matches("0x")));
    hex.push_str(&format!("{:064x}", request_offset));

    // ExitPoolRequest head
    hex.push_str(&format!("{:064x}", assets_offset));
    hex.push_str(&format!("{:064x}", min_amounts_offset));
    hex.push_str(&format!("{:064x}", user_data_offset));
    hex.push_str(&format!("{:064x}", 0u64)); // toInternalBalance = false

    // assets array
    hex.push_str(&format!("{:064x}", n));
    for token in tokens {
        hex.push_str(&format!("{:0>64}", token.trim_start_matches("0x")));
    }

    // minAmountsOut array
    hex.push_str(&format!("{:064x}", n));
    for &amt in min_amounts_out {
        hex.push_str(&format!("{:064x}", amt));
    }

    // userData
    hex.push_str(&format!("{:064x}", user_data_len));
    for chunk in user_data.chunks(32) {
        let mut padded = [0u8; 32];
        padded[..chunk.len()].copy_from_slice(chunk);
        hex.push_str(&hex::encode(padded));
    }

    hex
}
