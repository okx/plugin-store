use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct OpenVaultArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Collateral type: ETH-A, WBTC-A, USDC-A, WSTETH-A
    #[arg(long, default_value = "ETH-A")]
    pub ilk: String,

    /// Wallet address (optional, resolved from onchainos if omitted; use zero address for dry-run)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting (RECOMMENDED: CDP has liquidation risk)
    #[arg(long, default_value_t = true)]
    pub dry_run: bool,
}

pub async fn run(args: OpenVaultArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    let wallet = if args.dry_run {
        args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
    } else {
        match args.from.clone() {
            Some(a) => a,
            None => onchainos::resolve_wallet(chain_id)?,
        }
    };

    // Resolve ilk bytes32
    let ilk_hex = config::KNOWN_ILKS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(&args.ilk))
        .map(|(_, hex)| *hex)
        .ok_or_else(|| anyhow::anyhow!("Unknown ilk: {}. Supported: ETH-A, WBTC-A, USDC-A, WSTETH-A", args.ilk))?;

    let calldata = rpc::calldata_cdp_open(ilk_hex, &wallet);

    println!("=== Sky Lending — Open Vault (DRY-RUN) ===");
    println!("WARNING: CDP operations carry liquidation risk!");
    println!("This operation is dry-run only for safety.");
    println!();
    println!("From:     {}", wallet);
    println!("Ilk:      {}", args.ilk);
    println!("Contract: {}", config::CDP_MANAGER);
    println!("Calldata: {}", calldata);
    println!();
    println!("This would call: DssCdpManager.open(bytes32 ilk, address usr)");
    println!("  ilk  = {} (0x{})", args.ilk, ilk_hex);
    println!("  usr  = {}", wallet);
    println!();
    println!("Please confirm with the user before submitting any real transaction.");
    println!("[dry-run] Transaction NOT submitted.");

    Ok(())
}
