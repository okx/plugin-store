use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct DepositCollateralArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Amount of ETH to deposit as collateral (in ETH, e.g. 1.5)
    #[arg(long)]
    pub amount_eth: f64,

    /// Urn address (your vault's urn — get from 'sky-lending vaults')
    #[arg(long)]
    pub urn: Option<String>,

    /// Wallet address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting (REQUIRED: CDP has liquidation risk)
    #[arg(long, default_value_t = true)]
    pub dry_run: bool,
}

pub async fn run(args: DepositCollateralArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    let wallet = if args.dry_run {
        args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
    } else {
        match args.from.clone() {
            Some(a) => a,
            None => onchainos::resolve_wallet(chain_id)?,
        }
    };

    let urn = args.urn.as_deref().unwrap_or(&wallet);

    if args.amount_eth <= 0.0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let amount_wei = (args.amount_eth * 1e18) as u128;

    // EthJoin.join(address urn) — payable with ETH value
    let calldata = rpc::calldata_ethjoin_join(urn);

    println!("=== Sky Lending — Deposit Collateral (DRY-RUN) ===");
    println!("WARNING: CDP operations carry liquidation risk!");
    println!("This operation is dry-run only for safety.");
    println!();
    println!("From:      {}", wallet);
    println!("Urn:       {}", urn);
    println!("Amount:    {} ETH ({} wei)", args.amount_eth, amount_wei);
    println!("Contract:  {} (ETH-A Join)", config::ETH_A_JOIN);
    println!("Calldata:  {}", calldata);
    println!("ETH Value: {} wei", amount_wei);
    println!();
    println!("This would call: GemJoin.join(address urn)");
    println!("  with msg.value = {} wei ({} ETH)", amount_wei, args.amount_eth);
    println!();
    println!("After depositing collateral, use 'sky-lending draw-dai' to mint DAI.");
    println!("Please confirm with the user before submitting any real transaction.");
    println!("[dry-run] Transaction NOT submitted.");

    Ok(())
}
