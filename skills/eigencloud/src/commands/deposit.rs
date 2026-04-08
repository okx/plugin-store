use clap::Args;
use serde_json::json;

use crate::config::{
    build_approve_calldata, build_deposit_calldata, chain_id, resolve_token, rpc_url,
    strategy_manager,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{format_eth, get_allowance, get_erc20_balance};

#[derive(Args)]
pub struct DepositArgs {
    /// LST token symbol to deposit (stETH, rETH, cbETH, ETHx, osETH, wBETH, mETH, OETH, sfrxETH, lsETH, EIGEN)
    #[arg(long)]
    pub token: String,

    /// Amount to deposit in wei (e.g. 1000000000000000000 = 1 stETH)
    #[arg(long)]
    pub amount: String,

    /// Broadcast the transaction (required to execute; omit to preview)
    #[arg(long)]
    pub confirm: bool,

    /// Simulate without broadcasting (for testing)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: DepositArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let sm = strategy_manager();
    let cid = chain_id();

    // Parse amount as string — no f64
    let amount: u128 = args
        .amount
        .trim()
        .parse::<u128>()
        .map_err(|_| anyhow::anyhow!("Invalid amount: '{}' — must be a non-negative integer in wei", args.amount))?;

    if amount == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // Resolve token symbol
    let strat_info = resolve_token(&args.token)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown token '{}'. Supported: stETH, rETH, cbETH, ETHx, osETH, wBETH, mETH, OETH, sfrxETH, lsETH, EIGEN",
            args.token
        ))?;

    let wallet = resolve_wallet(cid)?;

    // Check token balance
    let balance = get_erc20_balance(strat_info.token, &wallet, rpc).await?;
    if !args.dry_run && balance < amount {
        anyhow::bail!(
            "Insufficient {} balance. Have {} wei, need {} wei",
            strat_info.symbol,
            balance,
            amount
        );
    }

    // Step 1: Approve StrategyManager if needed
    let allowance = get_allowance(strat_info.token, &wallet, sm, rpc).await?;
    let mut approve_tx = None;

    if !args.dry_run && allowance < amount {
        eprintln!(
            "WARNING: approving {} wei of {} ({}) to StrategyManager ({})",
            amount, strat_info.symbol, strat_info.token, sm
        );
        let approve_calldata = build_approve_calldata(sm, amount);
        let approve_result = wallet_contract_call(
            cid,
            strat_info.token,
            &approve_calldata,
            0,
            args.confirm,
            args.dry_run,
        )
        .await?;
        approve_tx = Some(extract_tx_hash(&approve_result).to_string());

        if args.confirm && !args.dry_run {
            // Brief wait for approval to land
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }

    // Step 2: Deposit into strategy
    let deposit_calldata = build_deposit_calldata(strat_info.strategy, strat_info.token, amount);

    let deposit_result = wallet_contract_call(
        cid,
        sm,
        &deposit_calldata,
        0,
        args.confirm,
        args.dry_run,
    )
    .await?;

    let tx_hash = extract_tx_hash(&deposit_result).to_string();

    let output = json!({
        "ok": true,
        "action": "deposit",
        "token": strat_info.symbol,
        "tokenAddress": strat_info.token,
        "strategyAddress": strat_info.strategy,
        "strategyManager": sm,
        "wallet": wallet,
        "amountWei": amount.to_string(),
        "amountFormatted": format_eth(amount),
        "approveTxHash": approve_tx,
        "depositTxHash": tx_hash,
        "confirmed": args.confirm,
        "dryRun": args.dry_run
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
