use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct UnstakeArgs {
    /// Amount of rsETH to unstake (in rsETH, not wei). Example: 0.05
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Wallet address (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: UnstakeArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    // Resolve wallet
    let wallet = if let Some(w) = args.from.clone() {
        w
    } else {
        let resolved = onchainos::resolve_wallet(chain_id, args.dry_run)?;
        if resolved.is_empty() || resolved == "0x" {
            anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
        }
        resolved
    };

    if args.amount <= 0.0 {
        anyhow::bail!("Unstake amount must be greater than 0");
    }
    let rs_eth_amount_wei = (args.amount * 1e18) as u128;

    // Fetch current rsETH/ETH price for display
    let price_calldata = rpc::calldata_no_params(config::SEL_RSETH_PRICE);
    let price_eth = match onchainos::eth_call(chain_id, config::ORACLE_ADDRESS, &price_calldata) {
        Ok(result) => match rpc::extract_return_data(&result) {
            Ok(hex) => rpc::decode_uint256(&hex).map(|p| p as f64 / 1e18).unwrap_or(1.0),
            Err(_) => 1.0,
        },
        Err(_) => 1.0,
    };
    let expected_eth = args.amount * price_eth;

    // Build calldata: initiateWithdrawal(address asset, uint256 rsEthAmount)
    // asset = ETH sentinel address
    let calldata = rpc::calldata_initiate_withdrawal(config::ETH_ASSET_ADDRESS, rs_eth_amount_wei);

    println!("=== Kelp DAO Unstake rsETH → ETH ===");
    println!("From:              {}", wallet);
    println!("rsETH Amount:      {} rsETH ({} wei)", args.amount, rs_eth_amount_wei);
    println!("Expected ETH:      ~{:.8} ETH (at current rate)", expected_eth);
    println!("rsETH/ETH Rate:    {:.8}", price_eth);
    println!("Contract:          {}", config::WITHDRAWAL_MANAGER_ADDRESS);
    println!("Calldata:          {}", calldata);
    println!();
    println!("Note: Withdrawals go through a queue and may take several days to finalize.");
    println!("      After initiating, you'll need to call completeWithdrawal once ready.");
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted. Calldata verified above.");
        return Ok(());
    }

    // Ask for confirmation (write operation)
    println!("⚠️  This will initiate withdrawal of {} rsETH from Kelp DAO.", args.amount);
    println!("   You will receive approximately {:.8} ETH after the unbonding period.", expected_eth);
    println!("   Please confirm this transaction. (Proceeding automatically via --force flag)");
    println!();

    println!("Submitting unstake transaction...");
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::WITHDRAWAL_MANAGER_ADDRESS,
        &calldata,
        Some(&wallet),
        None, // no ETH value for withdrawal initiation
        false,
    )
    .await?;

    if result["ok"].as_bool() == Some(false) || result["error"].is_string() {
        anyhow::bail!("Transaction failed: {}", result);
    }

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Withdrawal initiated: {}", tx_hash);
    println!();
    println!("Your withdrawal is now queued. Once finalized, call completeWithdrawal.");
    println!("Track your position: kelp positions --chain {}", chain_id);

    Ok(())
}
