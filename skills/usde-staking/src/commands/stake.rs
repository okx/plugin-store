use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of USDe to stake (e.g. 10.5)
    #[arg(long)]
    pub amount: f64,

    /// Wallet address (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run - show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    if args.amount <= 0.0 {
        anyhow::bail!("Stake amount must be greater than 0");
    }

    // Resolve wallet (after dry-run guard for zero-address safety)
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

    let amount_wei = rpc::float_to_wei(args.amount);

    // Preview: how many sUSDe will be received
    let preview_calldata = rpc::calldata_preview_deposit(amount_wei);
    let preview_result =
        onchainos::eth_call(config::CHAIN_ID, config::SUSDE_ADDRESS, &preview_calldata)?;
    let preview_hex = rpc::extract_return_data(&preview_result).unwrap_or_default();
    let expected_susde = rpc::decode_uint256(&preview_hex).unwrap_or(amount_wei);
    let expected_susde_float = rpc::wei_to_float(expected_susde);

    // Check USDe balance (skip for dry-run)
    if !args.dry_run {
        let bal_calldata = rpc::calldata_balance_of(&wallet);
        let bal_result =
            onchainos::eth_call(config::CHAIN_ID, config::USDE_ADDRESS, &bal_calldata)?;
        let bal_hex = rpc::extract_return_data(&bal_result).unwrap_or_default();
        let usde_balance = rpc::decode_uint256(&bal_hex).unwrap_or(0);
        if usde_balance < amount_wei {
            anyhow::bail!(
                "Insufficient USDe balance. Have {:.6} USDe, need {:.6} USDe.",
                rpc::wei_to_float(usde_balance),
                args.amount
            );
        }
    }

    // Build calldata for approve and deposit
    let approve_calldata = rpc::calldata_approve(config::SUSDE_ADDRESS, amount_wei);
    let deposit_calldata = rpc::calldata_deposit(amount_wei, &wallet);

    println!("=== USDe Staking ===");
    println!("From:              {}", wallet);
    println!("Amount:            {} USDe", args.amount);
    println!("Expected sUSDe:    {:.6} sUSDe", expected_susde_float);
    println!("Contract:          {}", config::SUSDE_ADDRESS);
    println!("Approve calldata:  {}", approve_calldata);
    println!("Deposit calldata:  {}", deposit_calldata);
    println!();

    if args.dry_run {
        println!("[dry-run] Transactions NOT submitted.");
        println!("Step 1 (approve): onchainos wallet contract-call --chain 1 --to {} --input-data {}",
            config::USDE_ADDRESS, approve_calldata);
        println!("Step 2 (deposit): onchainos wallet contract-call --chain 1 --to {} --input-data {}",
            config::SUSDE_ADDRESS, deposit_calldata);
        return Ok(());
    }

    // Step 1: Approve USDe spending
    // Ask user to confirm before submitting
    println!("This will stake {} USDe to receive ~{:.6} sUSDe.", args.amount, expected_susde_float);
    println!("Ask user to confirm before proceeding.");
    println!();
    println!("Step 1: Approving USDe spend...");
    let approve_result = onchainos::wallet_contract_call(
        config::CHAIN_ID,
        config::USDE_ADDRESS,
        &approve_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;
    let approve_hash = onchainos::extract_tx_hash(&approve_result);
    println!("Approve tx: {}", approve_hash);

    // Wait for approve to propagate before deposit (2-tx flow mitigation)
    println!("Waiting 15s for approve to confirm...");
    tokio::time::sleep(std::time::Duration::from_secs(15)).await;

    // Step 2: Deposit USDe → sUSDe
    println!("Step 2: Depositing USDe into sUSDe vault...");
    let deposit_result = onchainos::wallet_contract_call(
        config::CHAIN_ID,
        config::SUSDE_ADDRESS,
        &deposit_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;
    let deposit_hash = onchainos::extract_tx_hash(&deposit_result);
    println!("Deposit tx: {}", deposit_hash);
    println!();
    println!("Successfully staked {} USDe!", args.amount);
    println!("You received approximately {:.6} sUSDe.", expected_susde_float);
    println!("sUSDe automatically earns yield and appreciates against USDe.");
    println!("View on Etherscan: https://etherscan.io/tx/{}", deposit_hash);

    Ok(())
}
