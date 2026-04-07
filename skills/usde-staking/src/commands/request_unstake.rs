use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct RequestUnstakeArgs {
    /// Amount of sUSDe shares to unstake (e.g. 10.5). Use --by-assets to specify in USDe instead.
    #[arg(long, conflicts_with = "assets")]
    pub shares: Option<f64>,

    /// Amount of USDe assets to unstake (e.g. 10.0). Alternative to --shares.
    #[arg(long, conflicts_with = "shares")]
    pub assets: Option<f64>,

    /// Wallet address (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run - show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: RequestUnstakeArgs) -> anyhow::Result<()> {
    if args.shares.is_none() && args.assets.is_none() {
        anyhow::bail!("Specify either --shares <amount> or --assets <amount>");
    }

    // Resolve wallet (after dry-run guard)
    let wallet = if args.dry_run {
        args.from.clone().unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string())
    } else {
        args.from
            .clone()
            .unwrap_or_else(|| onchainos::resolve_wallet(config::CHAIN_ID).unwrap_or_default())
    };
    if !args.dry_run && wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Fetch cooldown duration
    let cd_calldata = rpc::calldata_cooldown_duration();
    let cd_result = onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &cd_calldata)?;
    let cd_hex = rpc::extract_return_data(&cd_result).unwrap_or_default();
    let cooldown_secs = rpc::decode_uint256(&cd_hex).unwrap_or(86400);
    let cooldown_days = cooldown_secs as f64 / 86400.0;

    // Determine calldata
    let (calldata, mode, amount_display) = if let Some(shares_float) = args.shares {
        if shares_float <= 0.0 {
            anyhow::bail!("Shares amount must be greater than 0");
        }
        let shares_wei = rpc::float_to_wei(shares_float);

        // Check sUSDe balance
        if !args.dry_run {
            let bal_calldata = rpc::calldata_balance_of(&wallet);
            let bal_result =
                onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &bal_calldata)?;
            let bal_hex = rpc::extract_return_data(&bal_result).unwrap_or_default();
            let susde_balance = rpc::decode_uint256(&bal_hex).unwrap_or(0);
            if susde_balance < shares_wei {
                anyhow::bail!(
                    "Insufficient sUSDe balance. Have {:.6} sUSDe, need {:.6} sUSDe.",
                    rpc::wei_to_float(susde_balance),
                    shares_float
                );
            }
        }

        (
            rpc::calldata_cooldown_shares(shares_wei),
            "shares",
            format!("{} sUSDe", shares_float),
        )
    } else {
        let assets_float = args.assets.unwrap();
        if assets_float <= 0.0 {
            anyhow::bail!("Assets amount must be greater than 0");
        }
        let assets_wei = rpc::float_to_wei(assets_float);
        (
            rpc::calldata_cooldown_assets(assets_wei),
            "assets",
            format!("{} USDe equivalent", assets_float),
        )
    };

    println!("=== Request Unstake (Initiate Cooldown) ===");
    println!("From:              {}", wallet);
    println!("Amount:            {}", amount_display);
    println!("Mode:              cooldown by {}", mode);
    println!("Cooldown period:   {:.0} day(s)", cooldown_days);
    println!("Contract:          {}", config::SUSDE_ADDRESS);
    println!("Calldata:          {}", calldata);
    println!();

    // NOTE: This operation initiates a cooldown period. Funds are locked during cooldown.
    // Mark as dry_run in context: no funds at risk during cooldown period testing.

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted (cooldown-gated op — dry-run mode).");
        println!("Command: onchainos wallet contract-call --chain 1 --to {} --input-data {}",
            config::SUSDE_ADDRESS, calldata);
        return Ok(());
    }

    // Ask user to confirm before initiating cooldown
    println!("WARNING: This will lock your sUSDe for {:.0} day(s) cooldown period.", cooldown_days);
    println!("Ask user to confirm before proceeding with request-unstake.");
    println!();
    println!("Submitting cooldown request...");

    let result = onchainos::wallet_contract_call(
        config::CHAIN_ID,
        config::SUSDE_ADDRESS,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Cooldown request tx: {}", tx_hash);
    println!();
    println!("Cooldown initiated for {}.", amount_display);
    println!("Your USDe will be claimable after {:.0} day(s).", cooldown_days);
    println!("Run 'usde-staking claim-unstake' after the cooldown period to receive USDe.");
    println!("View on Etherscan: https://etherscan.io/tx/{}", tx_hash);

    Ok(())
}
