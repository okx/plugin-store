use crate::{config, rpc};
use clap::Args;

#[derive(Args)]
pub struct RepayDaiArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Amount of DAI to repay (in DAI, e.g. 100.0)
    #[arg(long)]
    pub amount_dai: f64,

    /// Ilk type: ETH-A, WBTC-A, USDC-A, WSTETH-A
    #[arg(long, default_value = "ETH-A")]
    pub ilk: String,

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

pub async fn run(args: RepayDaiArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;
    let _ = chain_id; // used for context

    let wallet = args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string());
    let urn = args.urn.as_deref().unwrap_or(&wallet);

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
    let dart = -(amount_wei as i128);

    // Step 1: DAI.approve(DaiJoin, amount) — allow DaiJoin to pull DAI
    let approve_calldata = rpc::calldata_approve(config::DAI_JOIN, amount_wei);

    // Step 2: DaiJoin.join(urn, amount_wad) — burn DAI, credit Vat
    let join_calldata = rpc::calldata_daijoin_join(urn, amount_wei);

    // Step 3: Vat.frob(ilk, urn, urn, urn, 0, -dart) — reduce debt
    let frob_calldata = rpc::calldata_vat_frob(ilk_hex, urn, urn, urn, 0, dart);

    println!("=== Sky Lending — Repay DAI (DRY-RUN) ===");
    println!("WARNING: CDP operations carry liquidation risk!");
    println!("This operation is dry-run only for safety.");
    println!();
    println!("From:        {}", wallet);
    println!("Urn:         {}", urn);
    println!("Ilk:         {}", args.ilk);
    println!("DAI Amount:  {} DAI ({} wei)", args.amount_dai, amount_wei);
    println!();
    println!("Step 1 — DAI.approve (allow DaiJoin to pull DAI):");
    println!("  Contract:  {} (DAI Token)", config::DAI_TOKEN);
    println!("  Calldata:  {}", approve_calldata);
    println!();
    println!("Step 2 — DaiJoin.join (burn DAI):");
    println!("  Contract:  {} (DaiJoin)", config::DAI_JOIN);
    println!("  Calldata:  {}", join_calldata);
    println!();
    println!("Step 3 — Vat.frob (reduce debt):");
    println!("  Contract:  {} (Vat)", config::VAT);
    println!("  Calldata:  {}", frob_calldata);
    println!("  dart = {} (negative = repay)", dart);
    println!();
    println!("Please confirm with the user before submitting any real transaction.");
    println!("[dry-run] Transaction NOT submitted.");

    Ok(())
}
