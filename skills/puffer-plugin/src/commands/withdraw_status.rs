use clap::Args;
use serde_json::json;

use crate::config::{
    format_units, puffer_vault_address, rpc_url, withdrawal_manager_address, CHAIN_ID,
    WITHDRAWAL_BATCH_SIZE,
};
use crate::rpc::{convert_to_assets, get_finalized_batch, get_withdrawal, get_withdrawals_length};

#[derive(Args)]
pub struct WithdrawStatusArgs {
    /// Withdrawal index (returned by `request-withdraw`).
    #[arg(long)]
    pub id: u128,
}

pub async fn run(args: WithdrawStatusArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("withdraw-status")));
    }
    Ok(())
}

async fn run_inner(args: WithdrawStatusArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let manager = withdrawal_manager_address();
    let vault = puffer_vault_address();

    let finalized_batch = get_finalized_batch(manager, rpc).await?;
    let total_requests = get_withdrawals_length(manager, rpc).await?;
    let withdrawal = get_withdrawal(manager, args.id, rpc).await?;

    let batch_idx = args.id / (WITHDRAWAL_BATCH_SIZE as u128);
    let batch_finalized = batch_idx <= finalized_batch && finalized_batch != 0;
    let in_range = args.id < total_requests;

    let (puf_amount, rate_at_request, recipient, status, next_action, estimated_weth_out_raw): (
        String,
        String,
        String,
        &'static str,
        String,
        String,
    ) = match withdrawal {
        Some((puf, rate, recipient)) => {
            // Live preview of WETH at current rate (for information only — actual payout uses
            // the batch's locked rate once finalized).
            let live_eth = convert_to_assets(vault, puf, rpc).await.unwrap_or(0);
            let status = if batch_finalized {
                "CLAIMABLE"
            } else {
                "PENDING"
            };
            let next = if batch_finalized {
                format!(
                    "Run: puffer-plugin claim-withdraw --id {} --confirm",
                    args.id
                )
            } else {
                format!(
                    "Batch {} is not yet finalized (latest finalized = {}). Poll again later (~14 days from request).",
                    batch_idx, finalized_batch
                )
            };
            (
                format_units(puf, 18),
                format_units(rate, 18),
                recipient,
                status,
                next,
                live_eth.to_string(),
            )
        }
        None => {
            let (status, next) = if !in_range {
                (
                    "OUT_OF_RANGE",
                    format!(
                        "Withdrawal index {} does not exist. Total requests so far: {}.",
                        args.id, total_requests
                    ),
                )
            } else {
                (
                    "ALREADY_CLAIMED",
                    format!("Withdrawal {} has already been claimed (struct was cleared on `completeQueuedWithdrawal`).", args.id),
                )
            };
            (
                "0".into(),
                "0".into(),
                "0x0000000000000000000000000000000000000000".into(),
                status,
                next,
                "0".into(),
            )
        }
    };

    let out = json!({
        "ok": true,
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "withdrawal_id": args.id,
        "batch_index": batch_idx,
        "latest_finalized_batch": finalized_batch,
        "status": status,
        "is_claimable": status == "CLAIMABLE",
        "pufeth_amount": puf_amount,
        "pufeth_to_eth_rate_at_request": rate_at_request,
        "recipient": recipient,
        "estimated_weth_out_at_current_rate_raw": estimated_weth_out_raw,
        "next_action": next_action,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
