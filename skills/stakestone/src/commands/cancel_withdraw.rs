use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct CancelWithdrawArgs {
    /// Amount of STONE to cancel from withdrawal queue (e.g. 0.001)
    #[arg(long)]
    pub amount: f64,

    /// Wallet address (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run - show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: CancelWithdrawArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    if args.amount <= 0.0 {
        anyhow::bail!("Cancel amount must be greater than 0");
    }

    let shares_wei = (args.amount * 1e18) as u128;

    // Resolve wallet
    let wallet = if args.dry_run {
        args.from
            .clone()
            .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
    } else {
        match args.from.clone() {
            Some(f) => f,
            None => onchainos::resolve_wallet(chain_id)?,
        }
    };

    if !args.dry_run && wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Build calldata: cancelWithdraw(uint256 _shares)
    let calldata = format!(
        "0x{}{}",
        config::SEL_CANCEL_WITHDRAW,
        rpc::encode_uint256_u128(shares_wei)
    );

    println!("=== StakeStone Cancel Withdrawal ===");
    println!("From:             {}", wallet);
    println!("STONE to cancel:  {:.6} STONE ({} wei)", args.amount, shares_wei);
    println!("Contract:         {}", config::STONE_VAULT);
    println!("Calldata:         {}", calldata);
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted.");
        println!("calldata: {}", calldata);
        return Ok(());
    }

    // Ask user to confirm before submitting
    println!("This will cancel the withdrawal of {:.6} STONE from the queue.", args.amount);
    println!("The STONE will be returned to your wallet balance.");
    println!("Please confirm you want to submit this transaction.");
    println!();
    println!("Submitting cancel withdrawal...");

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::STONE_VAULT,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Transaction submitted: {}", tx_hash);
    println!("Your withdrawal has been cancelled. STONE returned to your wallet.");

    Ok(())
}
