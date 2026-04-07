use crate::{config, contracts::RocketPoolContracts, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct UnstakeArgs {
    /// Chain ID (default: 1 for Ethereum mainnet)
    #[arg(long, default_value_t = config::CHAIN_ID)]
    pub chain: u64,

    /// Amount of rETH to burn (e.g. 0.05)
    #[arg(long, allow_hyphen_values = true)]
    pub amount: f64,

    /// Wallet address to unstake from (resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run: show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: UnstakeArgs) -> anyhow::Result<()> {
    let chain_id = args.chain;

    // Validate amount
    if args.amount <= 0.0 {
        anyhow::bail!("Unstake amount must be greater than 0");
    }
    let reth_amount_wei = (args.amount * 1e18) as u128;

    // Resolve wallet
    let wallet = match args.from {
        Some(ref a) => a.clone(),
        None => onchainos::resolve_wallet(chain_id, args.dry_run)
            .map_err(|e| anyhow::anyhow!("Cannot resolve wallet: {}. Pass --from or ensure onchainos is logged in.", e))?,
    };

    // Resolve contracts
    let contracts = RocketPoolContracts::resolve(chain_id)?;

    // Get exchange rate
    let rate_calldata = format!("0x{}", config::SEL_GET_EXCHANGE_RATE);
    let rate_result = onchainos::eth_call(chain_id, &contracts.token_reth, &rate_calldata)?;
    let rate_data = rpc::extract_return_data(&rate_result)?;
    let rate_wei = rpc::decode_uint256(&rate_data).unwrap_or(1_000_000_000_000_000_000);
    let rate = rate_wei as f64 / 1e18;

    // Expected ETH output: ETH = rETH * rate
    let expected_eth = args.amount * rate;

    // Check rETH balance
    let balance_calldata = format!(
        "0x{}{}",
        config::SEL_BALANCE_OF,
        rpc::encode_address(&wallet)
    );
    let balance_result = onchainos::eth_call(chain_id, &contracts.token_reth, &balance_calldata)?;
    let balance_data = rpc::extract_return_data(&balance_result)?;
    let reth_balance = rpc::decode_uint256(&balance_data).unwrap_or(0);

    if !args.dry_run && reth_balance < reth_amount_wei {
        anyhow::bail!(
            "Insufficient rETH balance. Have: {:.6} rETH, Need: {:.6} rETH",
            reth_balance as f64 / 1e18,
            args.amount
        );
    }

    // Check deposit pool liquidity
    let pool_calldata = format!("0x{}", config::SEL_GET_DEPOSIT_BALANCE);
    let pool_result = onchainos::eth_call(chain_id, &contracts.deposit_pool, &pool_calldata)?;
    let pool_data = rpc::extract_return_data(&pool_result)?;
    let pool_balance = rpc::decode_uint256(&pool_data).unwrap_or(0);

    // Build burn(uint256) calldata
    let calldata = format!(
        "0x{}{}",
        config::SEL_BURN,
        rpc::encode_uint256_u128(reth_amount_wei)
    );

    println!("=== Rocket Pool Unstake ===");
    println!("From:             {}", wallet);
    println!("rETH to burn:     {} rETH ({} wei)", args.amount, reth_amount_wei);
    println!("Expected ETH:     ~{:.6} ETH", expected_eth);
    println!("Exchange rate:    1 rETH = {:.6} ETH", rate);
    println!("rETH contract:    {}", contracts.token_reth);
    println!("Deposit pool ETH: {:.4} ETH available", pool_balance as f64 / 1e18);
    println!("Calldata:         {}", calldata);
    println!();

    if pool_balance < (expected_eth * 1e18) as u128 && !args.dry_run {
        println!("WARNING: Deposit pool may have insufficient ETH liquidity.");
        println!("         Consider using a DEX (e.g. Uniswap) to swap rETH → ETH instead.");
        println!();
    }

    if args.dry_run {
        println!("[dry-run] Transaction NOT submitted.");
        println!("Would call: onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            chain_id, contracts.token_reth, calldata);
        return Ok(());
    }

    // IMPORTANT: Ask user to confirm before submitting
    println!("This will burn {} rETH and receive ~{:.6} ETH.", args.amount, expected_eth);
    println!("Please confirm the transaction details above before proceeding.");
    println!();
    println!("Submitting unstake transaction...");

    let result = onchainos::wallet_contract_call(
        chain_id,
        &contracts.token_reth,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("Transaction submitted: {}", tx_hash);
    println!("You will receive ~{:.6} ETH once the transaction is confirmed.", expected_eth);

    Ok(())
}
