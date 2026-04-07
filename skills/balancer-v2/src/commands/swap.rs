/// swap command — execute single swap via Vault.swap()

use anyhow::Result;
use serde::Serialize;
use tokio::time::{sleep, Duration};

use crate::config;
use crate::onchainos;
use crate::rpc;

#[derive(Debug, Serialize)]
struct SwapResult {
    tx_hash: String,
    pool_id: String,
    asset_in: String,
    asset_out: String,
    amount_in: String,
    min_amount_out: String,
    chain_id: u64,
    dry_run: bool,
}

pub async fn run(
    from_token: &str,
    to_token: &str,
    amount: f64,
    pool_id: &str,
    slippage_pct: f64, // e.g. 0.5 = 0.5%
    chain_id: u64,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::rpc_url(chain_id);
    let vault = config::VAULT_ADDRESS;
    let queries_contract = config::BALANCER_QUERIES_ADDRESS;

    let asset_in = config::resolve_token_address(from_token, chain_id);
    let asset_out = config::resolve_token_address(to_token, chain_id);

    let decimals_in = rpc::get_decimals(&asset_in, rpc_url).await.unwrap_or(18);
    let amount_in = (amount * 10f64.powi(decimals_in as i32)) as u128;

    // Get wallet address (needed for sender/recipient)
    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    // Get quote for min amount out
    let amount_out_expected = rpc::query_swap(
        queries_contract,
        pool_id,
        &asset_in,
        &asset_out,
        amount_in,
        rpc_url,
    )
    .await
    .unwrap_or(0);

    let min_amount_out = (amount_out_expected as f64 * (1.0 - slippage_pct / 100.0)) as u128;

    // Build Vault.swap calldata
    // swap((bytes32,uint8,address,address,uint256,bytes),(address,bool,address,bool),uint256,uint256)
    // selector: 0x52bbbe29
    let calldata = build_swap_calldata(
        pool_id,
        0u8, // GIVEN_IN
        &asset_in,
        &asset_out,
        amount_in,
        &wallet, // sender
        &wallet, // recipient — must be real wallet, never zero
        min_amount_out,
    );

    // Check allowance and approve if needed
    if !dry_run {
        let allowance = rpc::get_allowance(&asset_in, &wallet, vault, rpc_url).await?;
        if allowance < amount_in {
            eprintln!("Approving {} to Vault...", from_token);
            // Ask user to confirm is handled by SKILL.md description
            let approve_result = onchainos::erc20_approve(chain_id, &asset_in, vault, u128::MAX, None, false).await?;
            let approve_hash = onchainos::extract_tx_hash(&approve_result);
            eprintln!("Approve tx: {}", approve_hash);
            // Wait for approve to confirm before swap
            sleep(Duration::from_secs(3)).await;
        }
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        vault,
        &calldata,
        None,
        None,
        dry_run,
        true, // --force required for DEX swaps
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    let output = SwapResult {
        tx_hash,
        pool_id: pool_id.to_string(),
        asset_in: asset_in.clone(),
        asset_out: asset_out.clone(),
        amount_in: amount_in.to_string(),
        min_amount_out: min_amount_out.to_string(),
        chain_id,
        dry_run,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Build calldata for Vault.swap()
/// swap((bytes32,uint8,address,address,uint256,bytes),(address,bool,address,bool),uint256,uint256)
/// selector: 0x52bbbe29
fn build_swap_calldata(
    pool_id: &str,
    kind: u8,
    asset_in: &str,
    asset_out: &str,
    amount: u128,
    sender: &str,
    recipient: &str,
    limit: u128,
) -> String {
    let pool_id_clean = pool_id.trim_start_matches("0x");
    let asset_in_clean = asset_in.trim_start_matches("0x");
    let asset_out_clean = asset_out.trim_start_matches("0x");
    let sender_clean = sender.trim_start_matches("0x");
    let recipient_clean = recipient.trim_start_matches("0x");

    // ABI layout for swap(SingleSwap, FundManagement, uint256, uint256):
    // The top-level args are:
    //   arg0: offset to SingleSwap tuple (which has dynamic bytes) = 0x80 (4 * 32 bytes for 4 arg headers/offsets)
    //   arg1: offset to FundManagement tuple (all static) = 0x80 + singleSwap_size
    //   arg2: limit (uint256) — static, inline
    //   arg3: deadline (uint256) — static, inline
    //
    // SingleSwap has:
    //   poolId(32), kind(32), assetIn(32), assetOut(32), amount(32), bytes_offset(32) = 6 * 32 = 192 head bytes
    //   userData length=0 (32 bytes), no data
    //   Total = 7 * 32 = 224 bytes
    //
    // FundManagement: sender(32), fromInternalBalance(32), recipient(32), toInternalBalance(32) = 128 bytes
    //
    // deadline = now + 5 minutes as a large number
    let deadline = format!("{:064x}", u64::MAX); // max deadline for simplicity

    // Offsets from start of encoded args (not including selector):
    // arg0 = offset to singleSwap (pointer) = 4 * 32 = 128 = 0x80
    // arg1 = offset to fundManagement (pointer) — but FundManagement is all static, so it goes inline after singleSwap
    //   = 0x80 + 7 * 32 = 0x80 + 224 = 352 = 0x160
    // arg2 = limit (inline)
    // arg3 = deadline (inline)
    //
    // Wait - for top-level tuple args, we use "head" / "tail" encoding.
    // Top-level args (4 args):
    //   arg0 is a tuple (has dynamic member) → encoded as offset
    //   arg1 is a tuple (all static) → encoded as offset? NO - static tuples are inlined for top-level args
    //   Actually for top-level function args, each arg is encoded per ABI spec:
    //   - Static types are encoded in-place
    //   - Dynamic types (incl tuples with dynamic members) use offsets
    //
    //   SingleSwap has `bytes userData` which is dynamic → SingleSwap is a dynamic tuple → offset
    //   FundManagement has only static members → FundManagement is a static tuple → encoded in-place
    //
    // So actual top-level layout:
    //   [0..32]    offset to SingleSwap tail = ?
    //   [32..160]  FundManagement inline (4 * 32)
    //   [160..192] limit
    //   [192..224] deadline
    //   [224..]    SingleSwap tail data

    // Offset to SingleSwap: it comes after all inline args = 4 slots for top-level = ... wait
    // Top-level has 4 args. The "head" area is:
    //   arg0 (dynamic tuple) → 32-byte pointer
    //   arg1 (static tuple, 4 * 32 bytes) → 128 bytes inline
    //   arg2 (uint256) → 32 bytes
    //   arg3 (uint256) → 32 bytes
    // Total head = 32 + 128 + 32 + 32 = 224 bytes = 0xe0
    //
    // SingleSwap offset = 0xe0
    //
    // SingleSwap encoding (tail):
    //   poolId(32), kind(32), assetIn(32), assetOut(32), amount(32)
    //   bytes userData: offset within tuple = 5 * 32 = 0xa0
    //   userData length = 0
    //   Total = 7 * 32 = 224 bytes

    let singleswap_offset = format!("{:064x}", 0xe0u64);

    // FundManagement inline
    let fund_mgmt = format!(
        "{:0>64}{:064x}{:0>64}{:064x}",
        sender_clean,
        0u64, // fromInternalBalance = false
        recipient_clean,
        0u64, // toInternalBalance = false
    );

    // limit and deadline inline
    let limit_hex = format!("{:064x}", limit);

    // SingleSwap tail
    // bytes userData offset within tuple = 6 * 32 = 0xc0
    // (after the 6 head slots: poolId, kind, assetIn, assetOut, amount, this_offset_slot)
    let bytes_offset_within_tuple = format!("{:064x}", 6u64 * 32u64); // 0xc0
    let user_data_len = format!("{:064x}", 0u64);

    let singleswap_tail = format!(
        "{:0>64}{:064x}{:0>64}{:0>64}{}{}{} ",
        pool_id_clean,
        kind,
        asset_in_clean,
        asset_out_clean,
        rpc::pad_u256(amount),
        bytes_offset_within_tuple,
        user_data_len,
    );
    // Remove trailing space
    let singleswap_tail = singleswap_tail.trim();

    format!(
        "0x52bbbe29{}{}{}{}{}",
        singleswap_offset,
        fund_mgmt,
        limit_hex,
        deadline,
        singleswap_tail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_calldata_starts_with_selector() {
        let calldata = build_swap_calldata(
            "0x64541216bafffeec8ea535bb71fbc927831d0595000100000000000000000002",
            0,
            "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
            "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
            1000000000000000u128,
            "0x87fb0647faabea33113eaf1d80d67acb1c491b90",
            "0x87fb0647faabea33113eaf1d80d67acb1c491b90",
            0,
        );
        assert!(calldata.starts_with("0x52bbbe29"), "Wrong selector: {}", &calldata[..10]);
    }
}
