use crate::config;
use crate::onchainos;
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to stake (e.g. 0.00005)
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (only Ethereum mainnet supported)
    #[arg(long, default_value = "1")]
    pub chain: u64,

    /// Simulate without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

/// Stake ETH to receive frxETH via frxETHMinter.submit()
pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    // Dry-run guard must be before resolve_wallet
    if args.dry_run {
        let amt_wei = (args.amount * 1e18) as u128;
        // submit() — no args
        let calldata = format!("0x{}", config::SEL_SUBMIT);
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "calldata": calldata,
                "amount_eth": args.amount,
                "amount_wei": amt_wei.to_string(),
                "contract": config::FRXETH_MINTER
            })
        );
        return Ok(());
    }

    let wallet = onchainos::resolve_wallet(args.chain)?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Ensure onchainos is logged in.");
    }

    let amt_wei = (args.amount * 1e18) as u128;
    if amt_wei == 0 {
        anyhow::bail!("Amount too small (rounds to 0 wei)");
    }

    // submit() — no args, payable
    let calldata = format!("0x{}", config::SEL_SUBMIT);

    let result = onchainos::wallet_contract_call(
        args.chain,
        config::FRXETH_MINTER,
        &calldata,
        Some(&wallet),
        Some(amt_wei),
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "txHash": tx_hash,
                "action": "stake ETH → frxETH",
                "amount_eth": args.amount,
                "amount_wei": amt_wei.to_string(),
                "from": wallet,
                "contract": config::FRXETH_MINTER,
                "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
            }
        })
    );
    Ok(())
}
