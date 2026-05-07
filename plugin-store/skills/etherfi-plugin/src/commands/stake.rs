use clap::Args;
use crate::calldata::build_deposit_calldata;
use crate::config::{format_units, liquidity_pool_address, parse_units, rpc_url, CHAIN_ID};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::get_balance;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to deposit (e.g. "0.1", "1.5")
    #[arg(long)]
    pub amount: String,
    /// Dry run — build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    let rpc = rpc_url();
    let pool = liquidity_pool_address();

    // Parse ETH amount to wei (18 decimals)
    let eth_wei = parse_units(&args.amount, 18)?;

    if eth_wei == 0 {
        anyhow::bail!("Amount must be greater than zero.");
    }

    // ether.fi LiquidityPool enforces a minimum deposit of 0.001 ETH on-chain.
    // Catch it here to give a clear message instead of a cryptic on-chain revert.
    const MIN_STAKE_WEI: u128 = 1_000_000_000_000_000; // 0.001 ETH
    if eth_wei < MIN_STAKE_WEI {
        anyhow::bail!(
            "ether.fi minimum deposit is 0.001 ETH. Got {} ETH ({} wei). Please increase the amount.",
            args.amount, eth_wei
        );
    }

    // Resolve wallet address
    let wallet = resolve_wallet(CHAIN_ID)?;

    eprintln!("Staking {} ETH ({} wei) via LiquidityPool.deposit()", args.amount, eth_wei);
    eprintln!("  LiquidityPool: {}", pool);
    eprintln!("  Wallet: {}", wallet);
    eprintln!("  You will receive approximately {} eETH in return.", args.amount);
    eprintln!("  Run with --confirm to broadcast.");

    // Build deposit(address _referral) calldata
    // ETH value is passed as msg.value (native send), not ABI-encoded
    let calldata = build_deposit_calldata();

    let result = wallet_contract_call(
        CHAIN_ID,
        pool,
        &calldata,
        eth_wei,    // native ETH value in wei
        args.confirm,
        args.dry_run,
    )
    .await?;

    // In preview mode, print the preview and stop
    if result["preview"].as_bool() == Some(true) {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result);

    // Fetch updated eETH balance if live transaction
    let eeth_balance_str = if !args.dry_run && args.confirm {
        match get_balance(
            crate::config::eeth_address(),
            &wallet,
            rpc,
        )
        .await
        {
            Ok(bal) => format_units(bal, 18),
            Err(_) => "N/A".to_string(),
        }
    } else {
        "N/A".to_string()
    };

    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"action\":\"stake\",\"ethDeposited\":\"{}\",\"ethWei\":\"{}\",\"eETHBalance\":\"{}\"}}",
        tx_hash, args.amount, eth_wei, eeth_balance_str
    );

    Ok(())
}
