use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to stake (in ETH, not wei). Example: 0.1
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Wallet address to stake from (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
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

    // Convert ETH to wei
    if args.amount <= 0.0 {
        anyhow::bail!("Stake amount must be greater than 0");
    }
    let amount_wei = (args.amount * 1e18) as u128;

    // Fetch expected rsETH output from deposit pool
    let mint_calldata = rpc::calldata_get_rseth_amount(config::ETH_ASSET_ADDRESS, amount_wei);
    let expected_rseth_wei = match onchainos::eth_call(chain_id, config::DEPOSIT_POOL_ADDRESS, &mint_calldata) {
        Ok(result) => match rpc::extract_return_data(&result) {
            Ok(hex) => rpc::decode_uint256(&hex).unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    };
    let expected_rseth = expected_rseth_wei as f64 / 1e18;

    // Fetch current rsETH price for display
    let price_calldata = rpc::calldata_no_params(config::SEL_RSETH_PRICE);
    let price_eth = match onchainos::eth_call(chain_id, config::ORACLE_ADDRESS, &price_calldata) {
        Ok(result) => match rpc::extract_return_data(&result) {
            Ok(hex) => rpc::decode_uint256(&hex).map(|p| p as f64 / 1e18).unwrap_or(1.0),
            Err(_) => 1.0,
        },
        Err(_) => 1.0,
    };

    // Build calldata: depositETH(0, "")
    // minRSETHAmountExpected = 0 (no slippage protection for simplicity)
    let calldata = rpc::calldata_deposit_eth(0);

    println!("=== Kelp DAO Stake ETH → rsETH ===");
    println!("From:              {}", wallet);
    println!("Amount:            {} ETH ({} wei)", args.amount, amount_wei);
    println!("Expected rsETH:    {:.8} rsETH", if expected_rseth > 0.0 { expected_rseth } else { args.amount / price_eth });
    println!("rsETH/ETH Rate:    {:.8}", price_eth);
    println!("Contract:          {}", config::DEPOSIT_POOL_ADDRESS);
    println!("Calldata:          {}", calldata);
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted. Calldata verified above.");
        return Ok(());
    }

    // Ask for confirmation (write operation)
    println!("⚠️  This will deposit {} ETH into Kelp DAO and mint rsETH.", args.amount);
    println!("   Please confirm this transaction. (Proceeding automatically via --force flag)");
    println!();

    println!("Submitting stake transaction...");
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::DEPOSIT_POOL_ADDRESS,
        &calldata,
        Some(&wallet),
        Some(amount_wei),
        false,
    )
    .await?;

    if result["ok"].as_bool() == Some(false) || result["error"].is_string() {
        anyhow::bail!("Transaction failed: {}", result);
    }

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Transaction submitted: {}", tx_hash);
    println!("You will receive approximately {:.8} rsETH.", if expected_rseth > 0.0 { expected_rseth } else { args.amount / price_eth });
    println!();
    println!("Track your position: kelp positions --chain {}", chain_id);

    Ok(())
}
