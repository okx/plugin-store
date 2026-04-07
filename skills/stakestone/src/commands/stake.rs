use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to stake (e.g. 0.001)
    #[arg(long)]
    pub amount: f64,

    /// Wallet address to stake from (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run - show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    if args.amount <= 0.0 {
        anyhow::bail!("Stake amount must be greater than 0");
    }

    let amount_wei = (args.amount * 1e18) as u128;

    // Resolve wallet (skip for dry_run to avoid requiring login)
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

    // Fetch current share price to estimate output
    let price_hex = rpc::eth_call(
        config::STONE_VAULT,
        &format!("0x{}", config::SEL_CURRENT_SHARE_PRICE),
    )?;
    let price_raw = rpc::decode_uint256(&price_hex)?;
    let price_eth = price_raw as f64 / 1e18;
    let estimated_stone = args.amount / price_eth;

    // Calldata: deposit() — no parameters, ETH sent as value
    let calldata = format!("0x{}", config::SEL_DEPOSIT);

    println!("=== StakeStone Stake ===");
    println!("From:             {}", wallet);
    println!("ETH to stake:     {} ETH ({} wei)", args.amount, amount_wei);
    println!("Expected STONE:   {:.6} STONE", estimated_stone);
    println!("Exchange rate:    {:.6} ETH/STONE", price_eth);
    println!("Contract:         {}", config::STONE_VAULT);
    println!("Calldata:         {}", calldata);
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted.");
        println!("calldata: {}", calldata);
        return Ok(());
    }

    // Ask user to confirm before submitting
    println!("This will stake {} ETH and receive approximately {:.6} STONE.", args.amount, estimated_stone);
    println!("Please confirm you want to submit this transaction.");
    println!();
    println!("Submitting stake transaction...");

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::STONE_VAULT,
        &calldata,
        Some(&wallet),
        Some(amount_wei),
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Transaction submitted: {}", tx_hash);
    println!("You will receive approximately {:.6} STONE once the transaction confirms.", estimated_stone);
    println!("Note: STONE accrues yield via exchange rate appreciation (not rebasing).");

    Ok(())
}
