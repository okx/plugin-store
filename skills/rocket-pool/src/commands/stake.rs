use crate::{config, contracts::RocketPoolContracts, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Amount of ETH to stake (e.g. 0.1)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: f64,

    /// Wallet address to stake from (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run: show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    // Validate amount
    if args.amount <= 0.0 {
        anyhow::bail!("Stake amount must be greater than 0");
    }
    let amount_wei = (args.amount * 1e18) as u128;
    if amount_wei < config::MIN_DEPOSIT_WEI {
        anyhow::bail!(
            "Minimum deposit is 0.01 ETH ({} wei). Provided: {} ETH ({} wei)",
            config::MIN_DEPOSIT_WEI,
            args.amount,
            amount_wei
        );
    }

    // Resolve wallet
    let wallet = match args.from {
        Some(ref a) => a.clone(),
        None => onchainos::resolve_wallet(chain_id, args.dry_run)
            .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {}. Pass --from or ensure onchainos is logged in.", e))?,
    };

    // Resolve contracts
    let contracts = RocketPoolContracts::resolve(chain_id)?;

    // Get current exchange rate for display
    let rate_calldata = format!("0x{}", config::SEL_GET_EXCHANGE_RATE);
    let rate_result = onchainos::eth_call(chain_id, &contracts.token_reth, &rate_calldata)?;
    let rate_data = rpc::extract_return_data(&rate_result)?;
    let rate_wei = rpc::decode_uint256(&rate_data).unwrap_or(1_000_000_000_000_000_000);
    let rate = rate_wei as f64 / 1e18;

    // Calculate expected rETH output: rETH = ETH / rate
    let expected_reth = args.amount / rate;

    // deposit() calldata — no parameters, just the 4-byte selector
    let calldata = format!("0x{}", config::SEL_DEPOSIT);

    println!("=== Rocket Pool Stake ===");
    println!("From:              {}", wallet);
    println!("ETH to stake:      {} ETH ({} wei)", args.amount, amount_wei);
    println!("Expected rETH:     ~{:.6} rETH", expected_reth);
    println!("Exchange rate:     1 rETH = {:.6} ETH", rate);
    println!("Deposit contract:  {}", contracts.deposit_pool);
    println!("Calldata:          {}", calldata);
    println!();

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted.");
        println!("Would call: onchainos wallet contract-call --chain {} --to {} --amt {} --input-data {} --force",
            chain_id, contracts.deposit_pool, amount_wei, calldata);
        return Ok(());
    }

    // IMPORTANT: Ask user to confirm before submitting
    println!("This will deposit {} ETH into Rocket Pool and receive ~{:.6} rETH.", args.amount, expected_reth);
    println!("Please confirm the transaction details above before proceeding.");
    println!();
    println!("Submitting stake transaction...");

    let result = onchainos::wallet_contract_call(
        chain_id,
        &contracts.deposit_pool,
        &calldata,
        Some(&wallet),
        Some(amount_wei),
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Transaction submitted: {}", tx_hash);
    println!("You will receive ~{:.6} rETH once the transaction is confirmed.", expected_reth);
    println!("Check your rETH balance with: rocket-pool positions");

    Ok(())
}
