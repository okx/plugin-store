use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct ClaimUnstakeArgs {
    /// Receiver address for USDe (defaults to wallet address)
    #[arg(long)]
    pub receiver: Option<String>,

    /// Wallet address (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run - show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: ClaimUnstakeArgs) -> anyhow::Result<()> {
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

    let receiver = args.receiver.as_deref().unwrap_or(&wallet);

    // Check cooldown status
    if !args.dry_run {
        let cd_calldata = rpc::calldata_cooldowns(&wallet);
        let cd_result =
            onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &cd_calldata)?;
        let cd_hex = rpc::extract_return_data(&cd_result).unwrap_or_default();
        let (cooldown_end, underlying_amount) =
            rpc::decode_two_uint256(&cd_hex).unwrap_or((0, 0));

        if underlying_amount == 0 {
            anyhow::bail!("No pending unstake found for {}. Run 'request-unstake' first.", wallet);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u128;

        if cooldown_end > now {
            let remaining = cooldown_end - now;
            let hours = remaining / 3600;
            let mins = (remaining % 3600) / 60;
            anyhow::bail!(
                "Cooldown not yet complete. {} hours {} minutes remaining.",
                hours,
                mins
            );
        }

        let amount_float = rpc::wei_to_float(underlying_amount);
        println!("Pending unstake: {:.6} USDe — cooldown complete, ready to claim.", amount_float);
    }

    let calldata = rpc::calldata_unstake(receiver);

    println!("=== Claim Unstaked USDe ===");
    println!("Wallet:            {}", wallet);
    println!("Receiver:          {}", receiver);
    println!("Contract:          {}", config::SUSDE_ADDRESS);
    println!("Calldata:          {}", calldata);
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted.");
        println!("Command: onchainos wallet contract-call --chain 1 --to {} --input-data {}",
            config::SUSDE_ADDRESS, calldata);
        return Ok(());
    }

    // Ask user to confirm before claiming
    println!("This will transfer your unstaked USDe to {}.", receiver);
    println!("Ask user to confirm before proceeding with claim-unstake.");
    println!();
    println!("Submitting claim transaction...");

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
    println!("Claim tx: {}", tx_hash);
    println!();
    println!("Successfully claimed USDe!");
    println!("USDe transferred to: {}", receiver);
    println!("View on Etherscan: https://etherscan.io/tx/{}", tx_hash);

    Ok(())
}
