// claim — Claim finalized ETH withdrawal from Stader
// Write operation: requires user confirmation before broadcasting.
//
// Contract: UserWithdrawManager.claim(uint256 _requestId)
// Selector: 0x379607f5 (verified via cast sig)
// Prerequisites: withdrawal must be finalized (ethFinalized > 0)

use anyhow::Result;
use clap::Args;
use serde_json::json;
use crate::config;
use crate::rpc;
use crate::onchainos;

#[derive(Args)]
pub struct ClaimArgs {
    /// Withdrawal request ID to claim
    #[arg(long)]
    pub request_id: u128,
}

pub async fn execute(args: &ClaimArgs, rpc_url: &str, chain_id: u64, dry_run: bool) -> Result<()> {
    if dry_run {
        let request_id_hex = format!("{:064x}", args.request_id);
        let calldata = format!("0x379607f5{}", request_id_hex);
        let output = json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": calldata,
            "calldata_selector": "0x379607f5",
            "description": "claim(uint256) — claim finalized ETH withdrawal",
            "request_id": args.request_id
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let _wallet = onchainos::resolve_wallet(chain_id)?;

    // Check if request is finalized before attempting claim
    let withdraw_info = rpc::get_withdraw_request(rpc_url, config::USER_WITHDRAW_MANAGER, args.request_id).await?;

    if !withdraw_info.is_finalized {
        let output = json!({
            "ok": false,
            "error": "Withdrawal request is not yet finalized",
            "request_id": args.request_id,
            "eth_finalized": withdraw_info.eth_finalized,
            "note": "Stader withdrawal finalization typically takes 3-10 days. Check back later."
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Build claim calldata
    // claim(uint256 _requestId)
    // selector: 0x379607f5
    let request_id_hex = format!("{:064x}", args.request_id);
    let calldata = format!("0x379607f5{}", request_id_hex);

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::USER_WITHDRAW_MANAGER,
        &calldata,
        None,
        false,
    )?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    let output = json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "action": "claim",
            "request_id": args.request_id,
            "eth_claimed_wei": withdraw_info.eth_finalized,
            "eth_claimed": rpc::format_eth(withdraw_info.eth_finalized.parse::<u128>().unwrap_or(0)),
            "contract": config::USER_WITHDRAW_MANAGER,
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
