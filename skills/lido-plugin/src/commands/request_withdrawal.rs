use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct RequestWithdrawalArgs {
    /// Amount of stETH to withdraw in ETH (e.g. 1.5)
    #[arg(long)]
    pub amount_eth: f64,

    /// Wallet address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: RequestWithdrawalArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    // Resolve wallet address — must not be zero
    let wallet = match args.from.clone() {
        Some(f) => f,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Convert stETH to wei — round() avoids f64 truncation on values like 0.1 ETH
    let amount_wei = (args.amount_eth * 1e18_f64).round() as u128;
    if amount_wei < config::MIN_WITHDRAWAL_WEI {
        anyhow::bail!(
            "Withdrawal amount {} wei is below minimum {} wei",
            amount_wei,
            config::MIN_WITHDRAWAL_WEI
        );
    }
    if amount_wei > config::MAX_WITHDRAWAL_WEI {
        anyhow::bail!(
            "Withdrawal amount {} wei exceeds maximum {} wei (1000 ETH)",
            amount_wei,
            config::MAX_WITHDRAWAL_WEI
        );
    }

    // Build approve calldata: approve(WithdrawalQueueERC721, amount)
    let approve_calldata =
        rpc::calldata_approve(config::WITHDRAWAL_QUEUE_ADDRESS, amount_wei);

    // Build requestWithdrawals calldata
    let request_calldata = rpc::calldata_request_withdrawals(&[amount_wei], &wallet);

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "action": "requestWithdrawal",
            "from": wallet,
            "amountStEth": format!("{:.6}", args.amount_eth),
            "amountWei": amount_wei.to_string(),
            "step1_approve": {
                "contract": config::STETH_ADDRESS,
                "calldata": approve_calldata
            },
            "step2_request": {
                "contract": config::WITHDRAWAL_QUEUE_ADDRESS,
                "calldata": request_calldata
            },
            "note": "Add --confirm to broadcast. Withdrawal finalization typically takes 1–5 days."
        }));
        return Ok(());
    }

    // Pre-flight: stETH balance check (EVM-001)
    let balance_calldata = rpc::calldata_single_address(config::SEL_BALANCE_OF, &wallet);
    let balance_result = onchainos::eth_call(chain_id, config::STETH_ADDRESS, &balance_calldata).await
        .map_err(|e| anyhow::anyhow!("Failed to check stETH balance: {}", e))?;
    let steth_balance = rpc::extract_return_data(&balance_result)
        .and_then(|h| rpc::decode_uint256(&h))
        .map_err(|e| anyhow::anyhow!("Failed to decode stETH balance: {}", e))?;
    if steth_balance < amount_wei {
        anyhow::bail!(
            "Insufficient stETH balance: need {:.6} stETH, have {:.6} stETH.",
            amount_wei as f64 / 1e18,
            steth_balance as f64 / 1e18
        );
    }

    if !args.confirm {
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": true,
            "action": "requestWithdrawal",
            "from": wallet,
            "amountStEth": format!("{:.6}", args.amount_eth),
            "amountWei": amount_wei.to_string(),
            "note": "Add --confirm to execute. Withdrawal finalization typically takes 1–5 days."
        }));
        return Ok(());
    }

    // Step 1: Approve stETH spend — must be mined before step 2 can succeed
    eprintln!("Step 1/2: Approving stETH spend...");
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        config::STETH_ADDRESS,
        &approve_calldata,
        Some(&wallet),
        None,
        args.confirm,
        args.dry_run,
    )
    .await?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result, "Approve")?;
    eprintln!("Approve tx: {} — waiting for confirmation...", approve_tx);
    onchainos::wait_for_receipt(chain_id, &approve_tx, 120).await?;

    // Step 2: Request withdrawal
    eprintln!("Step 2/2: Submitting withdrawal request...");
    let request_result = onchainos::wallet_contract_call(
        chain_id,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &request_calldata,
        Some(&wallet),
        None,
        args.confirm,
        args.dry_run,
    )
    .await?;
    let request_tx = onchainos::extract_tx_hash_or_err(&request_result, "requestWithdrawals")?;
    eprintln!("Request tx: {} — waiting for confirmation...", request_tx);
    onchainos::wait_for_receipt(chain_id, &request_tx, 120).await?;

    println!("{}", serde_json::json!({
        "ok": true,
        "action": "requestWithdrawal",
        "from": wallet,
        "amountStEth": format!("{:.6}", args.amount_eth),
        "amountWei": amount_wei.to_string(),
        "approveTxHash": approve_tx,
        "requestTxHash": request_tx,
        "note": "Withdrawal request submitted. Use `lido get-withdrawals` to check status. Finalization typically takes 1–5 days."
    }));

    Ok(())
}
