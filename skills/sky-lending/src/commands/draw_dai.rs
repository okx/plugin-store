use crate::{config, rpc};
use clap::Args;

#[derive(Args)]
pub struct DrawDaiArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Amount of DAI to draw/mint (in DAI, e.g. 100.0)
    #[arg(long)]
    pub amount_dai: f64,

    /// Ilk type: ETH-A, WBTC-A, USDC-A, WSTETH-A
    #[arg(long, default_value = "ETH-A")]
    pub ilk: String,

    /// Urn address (your vault's urn — get from 'sky-lending vaults')
    #[arg(long)]
    pub urn: Option<String>,

    /// Recipient address for DAI (optional, defaults to from address)
    #[arg(long)]
    pub to: Option<String>,

    /// Wallet address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting (REQUIRED: CDP has liquidation risk)
    #[arg(long, default_value_t = true)]
    pub dry_run: bool,
}

pub async fn run(args: DrawDaiArgs) -> anyhow::Result<()> {
    let _chain_id = args.chain;

    let wallet = args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string());
    let urn = args.urn.as_deref().unwrap_or(&wallet);
    let recipient = args.to.as_deref().unwrap_or(&wallet);

    // Resolve ilk bytes32
    let ilk_hex = config::KNOWN_ILKS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&args.ilk))
        .map(|(_, hex)| *hex)
        .ok_or_else(|| anyhow::anyhow!("Unknown ilk: {}. Supported: ETH-A, WBTC-A, USDC-A, WSTETH-A", args.ilk))?;

    if args.amount_dai <= 0.0 {
        anyhow::bail!("DAI amount must be greater than 0");
    }

    let amount_wei = (args.amount_dai * 1e18) as u128;
    // dart in wad (1e18), but Vat uses normalized units — for simplicity treat as 1:1 (rate ~= 1)
    let dart = amount_wei as i128;

    // Step 1: Vat.frob(ilk, urn, urn, urn, 0, dart) — increase debt
    let frob_calldata = rpc::calldata_vat_frob(ilk_hex, urn, urn, urn, 0, dart);

    // Step 2: DaiJoin.exit(recipient, amount_wad) — mint DAI to wallet
    let exit_calldata = rpc::calldata_daijoin_exit(recipient, amount_wei);

    println!("=== Sky Lending — Draw DAI (DRY-RUN) ===");
    println!("WARNING: CDP operations carry liquidation risk!");
    println!("WARNING: Drawing DAI increases your debt and lowers your collateralization ratio.");
    println!("This operation is dry-run only for safety.");
    println!();
    println!("From:        {}", wallet);
    println!("Urn:         {}", urn);
    println!("Ilk:         {}", args.ilk);
    println!("DAI Amount:  {} DAI ({} wei)", args.amount_dai, amount_wei);
    println!("Recipient:   {}", recipient);
    println!();
    println!("Step 1 — Vat.frob (increase debt):");
    println!("  Contract:  {} (Vat)", config::VAT);
    println!("  Calldata:  {}", frob_calldata);
    println!("  dink=0, dart={}", dart);
    println!();
    println!("Step 2 — DaiJoin.exit (mint DAI):");
    println!("  Contract:  {} (DaiJoin)", config::DAI_JOIN);
    println!("  Calldata:  {}", exit_calldata);
    println!();
    println!("Please confirm with the user before submitting any real transaction.");
    println!("[dry-run] Transaction NOT submitted.");

    Ok(())
}
