use crate::{config, rpc};
use clap::Args;

#[derive(Args)]
pub struct WithdrawCollateralArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Amount of ETH collateral to withdraw (in ETH, e.g. 0.5)
    #[arg(long)]
    pub amount_eth: f64,

    /// Ilk type: ETH-A, WBTC-A, USDC-A, WSTETH-A
    #[arg(long, default_value = "ETH-A")]
    pub ilk: String,

    /// Urn address (your vault's urn — get from 'sky-lending vaults')
    #[arg(long)]
    pub urn: Option<String>,

    /// Recipient address for withdrawn collateral (optional, defaults to from address)
    #[arg(long)]
    pub to: Option<String>,

    /// Wallet address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting (REQUIRED: CDP has liquidation risk)
    #[arg(long, default_value_t = true)]
    pub dry_run: bool,
}

pub async fn run(args: WithdrawCollateralArgs) -> anyhow::Result<()> {
    let wallet = args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string());
    let urn = args.urn.as_deref().unwrap_or(&wallet);
    let recipient = args.to.as_deref().unwrap_or(&wallet);

    // Resolve ilk bytes32
    let ilk_hex = config::KNOWN_ILKS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&args.ilk))
        .map(|(_, hex)| *hex)
        .ok_or_else(|| anyhow::anyhow!("Unknown ilk: {}. Supported: ETH-A, WBTC-A, USDC-A, WSTETH-A", args.ilk))?;

    if args.amount_eth <= 0.0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let amount_wei = (args.amount_eth * 1e18) as u128;
    let dink = -(amount_wei as i128); // negative = remove collateral

    // Step 1: Vat.frob(ilk, urn, urn, urn, -dink, 0) — free collateral in Vat
    let frob_calldata = rpc::calldata_vat_frob(ilk_hex, urn, urn, urn, dink, 0);

    // Step 2: EthJoin.exit(recipient, amount_wad) — send ETH to user
    let exit_calldata = rpc::calldata_ethjoin_exit(recipient, amount_wei);

    println!("=== Sky Lending — Withdraw Collateral (DRY-RUN) ===");
    println!("WARNING: CDP operations carry liquidation risk!");
    println!("WARNING: Withdrawing collateral lowers your collateralization ratio.");
    println!("This operation is dry-run only for safety.");
    println!();
    println!("From:       {}", wallet);
    println!("Urn:        {}", urn);
    println!("Ilk:        {}", args.ilk);
    println!("Amount:     {} ETH ({} wei)", args.amount_eth, amount_wei);
    println!("Recipient:  {}", recipient);
    println!();
    println!("Step 1 — Vat.frob (free collateral):");
    println!("  Contract: {} (Vat)", config::VAT);
    println!("  Calldata: {}", frob_calldata);
    println!("  dink = {} (negative = remove collateral)", dink);
    println!();
    println!("Step 2 — EthJoin.exit (withdraw ETH):");
    println!("  Contract: {} (ETH-A Join)", config::ETH_A_JOIN);
    println!("  Calldata: {}", exit_calldata);
    println!();
    println!("Please confirm with the user before submitting any real transaction.");
    println!("[dry-run] Transaction NOT submitted.");

    Ok(())
}
