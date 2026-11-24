use clap::Args;
use serde_json::json;

use crate::calldata::build_complete_queued_withdrawal_calldata;
use crate::config::{
    format_units, rpc_url, weth_address, withdrawal_manager_address, CHAIN_ID,
    WITHDRAWAL_BATCH_SIZE,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wait_for_tx, wallet_balance, wallet_contract_call};
use crate::rpc::{get_finalized_batch, get_withdrawal, get_withdrawals_length};

#[derive(Args)]
pub struct ClaimWithdrawArgs {
    /// Withdrawal index (from `request-withdraw`).
    #[arg(long)]
    pub id: u128,
    /// Dry run — build calldata but do not broadcast.
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: ClaimWithdrawArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("claim-withdraw")));
    }
    Ok(())
}

async fn run_inner(args: ClaimWithdrawArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let manager = withdrawal_manager_address();
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Pre-flight: refuse to claim if the batch isn't finalized yet.
    let withdrawal = get_withdrawal(manager, args.id, rpc).await?;
    let withdrawal = match withdrawal {
        Some(w) => w,
        None => {
            let total = get_withdrawals_length(manager, rpc).await?;
            if args.id >= total {
                anyhow::bail!(
                    "Withdrawal index {} does not exist (total requests so far: {}).",
                    args.id, total
                );
            } else {
                anyhow::bail!(
                    "WithdrawalAlreadyCompleted: withdrawal {} has already been claimed — struct was cleared on-chain.",
                    args.id
                );
            }
        }
    };
    let (puf_amount, _rate, recipient) = withdrawal;

    let finalized_batch = get_finalized_batch(manager, rpc).await?;
    let batch_idx = args.id / (WITHDRAWAL_BATCH_SIZE as u128);
    if batch_idx > finalized_batch {
        anyhow::bail!(
            "BatchNotFinalized: withdrawal {} is in batch {} but latest finalized batch = {}. Run withdraw-status --id {} to monitor.",
            args.id, batch_idx, finalized_batch, args.id
        );
    }

    let calldata = build_complete_queued_withdrawal_calldata(args.id);

    // Gas budget check (no value sent on a claim).
    let gas = super::check_gas_budget(&wallet, manager, &calldata, 0, rpc).await?;

    eprintln!(
        "Claiming withdrawal {} (recipient={}, amount={} pufETH)",
        args.id,
        recipient,
        format_units(puf_amount, 18)
    );
    eprintln!(
        "  Gas: ~{} units × {} gwei = {} ETH (wallet has {} ETH)",
        gas.gas_units,
        format_units(gas.gas_price_wei, 9),
        format_units(gas.estimated_fee_wei, 18),
        format_units(gas.wallet_eth_balance_wei, 18),
    );

    let result = wallet_contract_call(
        CHAIN_ID,
        manager,
        &calldata,
        0,
        args.confirm,
        args.dry_run,
    )
    .await?;

    if result["preview"].as_bool() == Some(true) || result["dry_run"].as_bool() == Some(true) {
        let out = json!({
            "ok": true,
            "action": "claim-withdraw",
            "step": "preview",
            "chain": "ethereum",
            "chain_id": CHAIN_ID,
            "withdrawal_id": args.id,
            "batch_index": batch_idx,
            "latest_finalized_batch": finalized_batch,
            "pufeth_amount": format_units(puf_amount, 18),
            "pufeth_amount_raw": puf_amount.to_string(),
            "recipient": recipient,
            "withdrawal_manager": manager,
            "calldata": calldata,
            "gas_check": gas.to_json(),
            "next_action": "Re-run with --confirm to broadcast.",
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result).to_string();
    eprintln!("Claim tx: {} — waiting for confirmation...", tx_hash);
    wait_for_tx(tx_hash.clone(), wallet.clone()).await?;
    eprintln!("Claim confirmed.");
    // Post-tx balance read with cache bypass so we don't read pre-claim WETH balance.
    let weth_bal = wallet_balance(CHAIN_ID, Some(weth_address()), true).await.unwrap_or(0);

    let out = json!({
        "ok": true,
        "action": "claim-withdraw",
        "step": "2 of 2 (claimed)",
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "tx_hash": tx_hash,
        "withdrawal_id": args.id,
        "batch_index": batch_idx,
        "pufeth_amount": format_units(puf_amount, 18),
        "pufeth_amount_raw": puf_amount.to_string(),
        "recipient": recipient,
        "weth_balance_after": format_units(weth_bal, 18),
        "weth_balance_after_raw": weth_bal.to_string(),
        "gas_check": gas.to_json(),
        "note": "WETH has been transferred to the recipient. Unwrap WETH→ETH separately if needed.",
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
