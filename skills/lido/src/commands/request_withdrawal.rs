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
    let wallet = args
        .from
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let amount_wei = (args.amount_eth * 1e18) as u128;
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

    println!("=== Lido Request Withdrawal ===");
    println!("From:        {}", wallet);
    println!("Amount:      {} stETH ({} wei)", args.amount_eth, amount_wei);
    println!("Step 1:      Approve stETH to WithdrawalQueueERC721");
    println!("  Contract:  {}", config::STETH_ADDRESS);
    println!("  Calldata:  {}", approve_calldata);
    println!("Step 2:      Submit requestWithdrawals");
    println!("  Contract:  {}", config::WITHDRAWAL_QUEUE_ADDRESS);
    println!("  Calldata:  {}", request_calldata);
    println!();
    println!(
        "Warning: Withdrawal finalization typically takes 1-5 days (longer during Bunker mode)."
    );

    if args.dry_run {
        println!("[dry-run] Transactions NOT submitted.");
        return Ok(());
    }

    if !args.confirm {
        println!("=== Transaction Preview (NOT broadcast) ===");
        println!("Add --confirm to execute this transaction.");
        return Ok(());
    }

    // Step 1: Approve
    println!("Step 1/2: Approving stETH spend...");
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
    let approve_tx = onchainos::extract_tx_hash(&approve_result);
    println!("Approve tx: {}", approve_tx);

    // Step 2: Request withdrawal
    println!("Step 2/2: Submitting withdrawal request...");
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
    let request_tx = onchainos::extract_tx_hash(&request_result);
    println!("Request tx: {}", request_tx);
    println!();
    println!("Withdrawal request submitted. You will receive an unstETH NFT (ERC-721).");
    println!("Use `lido get-withdrawals` to check status.");

    Ok(())
}
