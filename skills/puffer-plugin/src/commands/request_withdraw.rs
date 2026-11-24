use clap::Args;
use serde_json::json;

use crate::calldata::{build_approve_calldata, build_request_withdrawal_calldata};
use crate::config::{
    format_units, parse_units, pufeth_address, puffer_vault_address, rpc_url,
    withdrawal_manager_address, CHAIN_ID, MIN_WITHDRAWAL_AMOUNT_WEI, WITHDRAWAL_BATCH_SIZE,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wait_for_tx, wallet_balance, wallet_contract_call};
use crate::rpc::{
    convert_to_assets, get_allowance, get_finalized_batch, get_max_withdrawal_amount,
    get_withdrawals_length,
};

#[derive(Args)]
pub struct RequestWithdrawArgs {
    /// Amount of pufETH to queue for withdrawal (e.g. "0.1"). Must be ≥ 0.01.
    #[arg(long)]
    pub amount: String,
    /// Dry run — build calldata but do not broadcast.
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction(s). Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: RequestWithdrawArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("request-withdraw")));
    }
    Ok(())
}

async fn run_inner(args: RequestWithdrawArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let vault = puffer_vault_address();
    let manager = withdrawal_manager_address();
    let pufeth = pufeth_address();

    let amount_raw = parse_units(&args.amount, 18)?;
    if amount_raw < MIN_WITHDRAWAL_AMOUNT_WEI {
        anyhow::bail!(
            "WithdrawalAmountTooLow: requested {} pufETH is below the 0.01 pufETH minimum. Use instant-withdraw instead.",
            format_units(amount_raw, 18)
        );
    }

    // Per-request maximum set by governance. Queried live so skill stays correct after policy changes.
    let max_per_request = get_max_withdrawal_amount(manager, rpc).await?;
    if amount_raw > max_per_request {
        anyhow::bail!(
            "WithdrawalAmountTooHigh: requested {} pufETH exceeds the per-request maximum {} pufETH. Split the request, or use instant-withdraw.",
            format_units(amount_raw, 18),
            format_units(max_per_request, 18)
        );
    }

    let wallet = resolve_wallet(CHAIN_ID)?;

    // Pre-flight pufETH balance check via onchainos (→ EVM-001).
    let bal_raw = wallet_balance(CHAIN_ID, Some(pufeth), false).await?;
    if bal_raw < amount_raw {
        anyhow::bail!(
            "Insufficient pufETH balance: need {}, have {}.",
            format_units(amount_raw, 18),
            format_units(bal_raw, 18)
        );
    }

    // Quote gross ETH value (no fee on 2-step path)
    let est_weth_raw = convert_to_assets(vault, amount_raw, rpc).await?;

    // Snapshot queue length — the new withdrawal index will be exactly this value on success.
    let queue_len_before = get_withdrawals_length(manager, rpc).await?;
    let assigned_idx = queue_len_before;
    let finalized_batch = get_finalized_batch(manager, rpc).await?;
    let batch_idx = assigned_idx / (WITHDRAWAL_BATCH_SIZE as u128);

    let request_calldata = build_request_withdrawal_calldata(amount_raw, &wallet);
    let approve_calldata = build_approve_calldata(manager, amount_raw);

    eprintln!("Requesting 2-step withdrawal of {} pufETH", format_units(amount_raw, 18));
    eprintln!("  WithdrawalManager: {}", manager);
    eprintln!("  Recipient: {}", wallet);
    eprintln!("  Estimated WETH out at finalization: ~{} (no fee)", format_units(est_weth_raw, 18));
    eprintln!("  Expected withdrawalId on success: {}", assigned_idx);
    eprintln!("  Expected batch index: {}", batch_idx);

    // Need allowance for manager to pull pufETH via transferFrom in _processWithdrawalRequest.
    let current_allowance = get_allowance(pufeth, &wallet, manager, rpc).await?;
    let needs_approve = current_allowance < amount_raw;

    // Gas pre-flight. eth_estimateGas on the request call would revert when allowance is
    // missing, so use a conservative static cap: 60k (approve, if needed) + 250k (request
    // transferFrom + struct write). 1.2x drift buffer is applied inside check_gas_budget_cap.
    const APPROVE_GAS_CAP: u128 = 60_000;
    const REQUEST_GAS_CAP: u128 = 250_000;
    let gas_cap = if needs_approve { APPROVE_GAS_CAP + REQUEST_GAS_CAP } else { REQUEST_GAS_CAP };
    let gas = super::check_gas_budget_cap(&wallet, gas_cap, 0, rpc).await?;

    // Preview branch
    if args.dry_run || !args.confirm {
        let out = json!({
            "ok": true,
            "action": "request-withdraw",
            "step": "preview",
            "chain": "ethereum",
            "chain_id": CHAIN_ID,
            "pufeth_amount": format_units(amount_raw, 18),
            "pufeth_amount_raw": amount_raw.to_string(),
            "recipient": wallet,
            "withdrawal_manager": manager,
            "estimated_weth_out": format_units(est_weth_raw, 18),
            "estimated_weth_out_raw": est_weth_raw.to_string(),
            "estimated_finalization_days": 14,
            "fee_pct": 0,
            "needs_approve": needs_approve,
            "current_allowance_raw": current_allowance.to_string(),
            "gas_check": gas.to_json(),
            "max_amount_per_request_pufeth": format_units(max_per_request, 18),
            "expected_withdrawal_id": assigned_idx,
            "expected_batch_index": batch_idx,
            "latest_finalized_batch": finalized_batch,
            "approve_calldata": approve_calldata,
            "request_calldata": request_calldata,
            "next_action": "Re-run with --confirm to broadcast (approve + request).",
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Step A: approve if needed
    if needs_approve {
        let result = wallet_contract_call(
            CHAIN_ID,
            pufeth,
            &approve_calldata,
            0,
            true,
            false,
        )
        .await?;
        let approve_hash = extract_tx_hash(&result).to_string();
        eprintln!("Approve tx: {} — waiting for confirmation...", approve_hash);
        wait_for_tx(approve_hash.clone(), wallet.clone()).await?;
        eprintln!("Approve confirmed.");
    }

    // Step B: request withdrawal
    let result = wallet_contract_call(
        CHAIN_ID,
        manager,
        &request_calldata,
        0,
        true,
        false,
    )
    .await?;
    let tx_hash = extract_tx_hash(&result).to_string();

    // Confirm the submit tx before trusting assigned_idx (prevents race where another user's
    // request lands between our snapshot and our tx).
    wait_for_tx(tx_hash.clone(), wallet.clone()).await?;

    // Re-read queue length to verify the index we computed matches reality. If multiple
    // requests landed in the same block, we pick the most recent one with our recipient.
    let queue_len_after = get_withdrawals_length(manager, rpc).await?;
    let assumed_idx = queue_len_before;
    let idx_confirmed = queue_len_after > queue_len_before;

    let out = json!({
        "ok": true,
        "action": "request-withdraw",
        "step": "1 of 2 (request submitted)",
        "chain": "ethereum",
        "chain_id": CHAIN_ID,
        "tx_hash": tx_hash,
        "pufeth_amount": format_units(amount_raw, 18),
        "pufeth_amount_raw": amount_raw.to_string(),
        "recipient": wallet,
        "estimated_weth_out": format_units(est_weth_raw, 18),
        "estimated_weth_out_raw": est_weth_raw.to_string(),
        "fee_pct": 0,
        "estimated_finalization_days": 14,
        "withdrawal_id": assumed_idx,
        "batch_index": batch_idx,
        "withdrawal_id_confirmed": idx_confirmed,
        "gas_check": gas.to_json(),
        "latest_finalized_batch": finalized_batch,
        "next_action": format!("After ~14 days, run: puffer-plugin claim-withdraw --id {} --confirm", assumed_idx),
        "hint": "Poll with `puffer-plugin withdraw-status --id {id}` to check when the batch is finalized.",
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
