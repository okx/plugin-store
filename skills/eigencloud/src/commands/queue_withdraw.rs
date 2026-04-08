use clap::Args;
use serde_json::json;

use crate::config::{chain_id, delegation_manager, pad_address, pad_u256, resolve_token, rpc_url};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};

#[derive(Args)]
pub struct QueueWithdrawArgs {
    /// LST token symbol to withdraw (stETH, rETH, cbETH, ETHx, osETH, wBETH, mETH, OETH, sfrxETH, lsETH, EIGEN)
    #[arg(long)]
    pub token: String,

    /// Amount of shares to withdraw in wei (use 'all' to withdraw all shares)
    #[arg(long)]
    pub shares: String,

    /// Broadcast the transaction (required to execute; omit to preview)
    #[arg(long)]
    pub confirm: bool,

    /// Simulate without broadcasting (for testing)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: QueueWithdrawArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let dm = delegation_manager();
    let cid = chain_id();

    // Resolve token symbol
    let strat_info = resolve_token(&args.token)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown token '{}'. Supported: stETH, rETH, cbETH, ETHx, osETH, wBETH, mETH, OETH, sfrxETH, lsETH, EIGEN",
            args.token
        ))?;

    let wallet = resolve_wallet(cid)?;

    // Get current deposit shares for this strategy
    let current_shares = crate::rpc::get_staker_strategy_shares(
        crate::config::strategy_manager(),
        &wallet,
        strat_info.strategy,
        rpc,
    )
    .await?;

    // Parse shares amount
    let withdraw_shares: u128 = if args.shares.trim().to_lowercase() == "all" {
        if current_shares == 0 {
            anyhow::bail!("No {} shares to withdraw", strat_info.symbol);
        }
        current_shares
    } else {
        let s: u128 = args.shares.trim().parse().map_err(|_| {
            anyhow::anyhow!("Invalid shares amount: '{}' — must be a non-negative integer or 'all'", args.shares)
        })?;
        if !args.dry_run && s > current_shares {
            anyhow::bail!(
                "Insufficient shares: have {} wei, requested {} wei",
                current_shares,
                s
            );
        }
        s
    };

    // Build queueWithdrawals calldata
    // queueWithdrawals(QueuedWithdrawalParams[] params)
    // QueuedWithdrawalParams { address[] strategies; uint256[] shares; address withdrawer }
    // Selector: 0x0dd8dd02
    //
    // ABI encoding for a 1-element array with 1-element inner arrays:
    //   [0] outer array offset = 0x20
    //   [1] outer array length = 1
    //   [2] struct offset = 0x20 (relative to start of array data)
    //   -- struct fields --
    //   [3] strategies offset (within struct) = 0x60
    //   [4] shares offset (within struct) = 0xa0
    //   [5] withdrawer = wallet
    //   [6] strategies length = 1
    //   [7] strategy address
    //   [8] shares length = 1
    //   [9] shares amount
    let calldata = build_queue_withdraw_calldata(strat_info.strategy, withdraw_shares, &wallet);

    let result = wallet_contract_call(cid, dm, &calldata, 0, args.confirm, args.dry_run).await?;
    let tx_hash = extract_tx_hash(&result).to_string();

    let output = json!({
        "ok": true,
        "action": "queue-withdraw",
        "token": strat_info.symbol,
        "strategyAddress": strat_info.strategy,
        "sharesQueuedWei": withdraw_shares.to_string(),
        "currentSharesWei": current_shares.to_string(),
        "wallet": wallet,
        "delegationManager": dm,
        "note": "Withdrawal has a 7-day delay before it can be completed. Use complete-withdraw after the delay.",
        "txHash": tx_hash,
        "confirmed": args.confirm,
        "dryRun": args.dry_run
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Build queueWithdrawals calldata for a single strategy withdrawal.
/// queueWithdrawals(QueuedWithdrawalParams[] params) — selector 0x0dd8dd02
fn build_queue_withdraw_calldata(strategy: &str, shares: u128, withdrawer: &str) -> String {
    // outer array: offset to array start (after selector)
    let outer_offset = pad_u256(0x20);
    let outer_len = pad_u256(1);
    // struct offset within array (relative to array data start)
    let struct_offset = pad_u256(0x20);
    // Within struct: strategies is at offset 0x60 (3 words), shares at 0xa0 (5 words)
    let strategies_offset = pad_u256(0x60);
    let shares_offset = pad_u256(0xa0);
    let withdrawer_padded = pad_address(withdrawer);
    let strategies_len = pad_u256(1);
    let strategy_padded = pad_address(strategy);
    let shares_len = pad_u256(1);
    let shares_padded = pad_u256(shares);

    format!(
        "0x0dd8dd02{}{}{}{}{}{}{}{}{}{}",
        outer_offset,
        outer_len,
        struct_offset,
        strategies_offset,
        shares_offset,
        withdrawer_padded,
        strategies_len,
        strategy_padded,
        shares_len,
        shares_padded,
    )
}
