use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct RequestWithdrawArgs {
    /// Amount of STONE to withdraw (e.g. 0.001)
    #[arg(long)]
    pub amount: f64,

    /// Wallet address (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run - show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: RequestWithdrawArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    if args.amount <= 0.0 {
        anyhow::bail!("Withdraw amount must be greater than 0");
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

    // Fetch current price for ETH estimate
    let price_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_CURRENT_SHARE_PRICE),
    )?;
    let price_raw = rpc::decode_uint256(&price_hex)?;
    let price_eth = price_raw as f64 / 1e18;
    let estimated_eth = args.amount * price_eth;

    // Fetch withdrawal fee
    let fee_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_WITHDRAW_FEE_RATE),
    )?;
    let fee_raw = rpc::decode_uint256(&fee_hex)?;
    let fee_pct = fee_raw as f64 / 1e6 * 100.0;

    // Check STONE balance (skip for dry_run)
    if !args.dry_run {
        let calldata_balance = format!(
            "0x{}{}",
            config::SEL_BALANCE_OF,
            rpc::encode_address(&wallet)
        );
        let balance_hex = rpc::eth_call(config::STONE_TOKEN, &calldata_balance)?;
        let stone_balance = rpc::decode_uint256(&balance_hex)?;
        if shares_wei > stone_balance {
            anyhow::bail!(
                "Insufficient STONE balance: have {:.6} STONE, need {:.6} STONE",
                stone_balance as f64 / 1e18,
                args.amount
            );
        }
    }

    // Build calldata: requestWithdraw(uint256 _shares)
    let calldata = format!(
        "0x{}{}",
        config::SEL_REQUEST_WITHDRAW,
        rpc::encode_uint256_u128(shares_wei)
    );

    println!("=== StakeStone Request Withdrawal ===");
    println!("From:             {}", wallet);
    println!("STONE to queue:   {:.6} STONE ({} wei)", args.amount, shares_wei);
    println!("Est. ETH return:  {:.6} ETH (at {:.6} ETH/STONE)", estimated_eth, price_eth);
    println!("Withdrawal fee:   {:.4}%", fee_pct);
    println!("Contract:         {}", config::STONE_VAULT);
    println!("Calldata:         {}", calldata);
    println!();
    println!("Note: Withdrawal is processed in settlement rounds. ETH is released after the");
    println!("next round settles. Check your position with 'stakestone get-position'.");
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted.");
        println!("calldata: {}", calldata);
        return Ok(());
    }

    // Ask user to confirm before submitting
    println!("This will queue {:.6} STONE for withdrawal (~{:.6} ETH).", args.amount, estimated_eth);
    println!("Please confirm you want to submit this transaction.");
    println!();
    println!("Submitting withdrawal request...");

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
    println!("Your withdrawal request has been queued. Monitor with 'stakestone get-position'.");

    Ok(())
}
