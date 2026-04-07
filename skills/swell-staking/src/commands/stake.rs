use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to stake (human-readable, e.g. "0.001")
    #[arg(long)]
    pub amount: String,

    /// Wallet address to stake from (optional; resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Simulate without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs, chain_id: u64) -> anyhow::Result<()> {
    let amount_wei = rpc::parse_eth_to_wei(&args.amount)?;
    if amount_wei == 0 {
        anyhow::bail!("Stake amount must be greater than 0");
    }

    // Build calldata: deposit() — selector only, payable, no parameters
    let calldata = rpc::calldata_noarg(config::SEL_DEPOSIT);

    // Show preview (before wallet resolution for dry-run efficiency)
    eprintln!("=== Swell Stake (swETH) ===");
    eprintln!("Amount:    {} ETH ({} wei)", args.amount, amount_wei);
    eprintln!("Contract:  {} (swETH)", config::SWETH_ADDRESS);
    eprintln!("Calldata:  {}", calldata);
    eprintln!();

    if args.dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": calldata,
            "amount_wei": amount_wei.to_string()
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Resolve wallet — only needed for live tx
    let wallet = match args.from {
        Some(ref f) => f.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Pre-flight: fetch current rate to display expected swETH output
    let eth_to_sweth_calldata = rpc::calldata_noarg(config::SEL_ETH_TO_SWETH_RATE);
    let rate_result = onchainos::eth_call(chain_id, config::SWETH_ADDRESS, &eth_to_sweth_calldata);
    if let Ok(rate_raw) = rate_result {
        if let Ok(return_data) = rpc::extract_return_data(&rate_raw) {
            if let Ok(rate) = rpc::decode_uint256(&return_data) {
                let expected_sweth = (amount_wei as u128).saturating_mul(rate) / 1_000_000_000_000_000_000u128;
                eprintln!("Expected swETH: ~{}", rpc::format_eth(expected_sweth));
                eprintln!("From:           {}", wallet);
            }
        }
    }
    eprintln!();
    eprintln!("Ask user to confirm before proceeding.");
    eprintln!();

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::SWETH_ADDRESS,
        &calldata,
        Some(&wallet),
        Some(amount_wei),
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let output = serde_json::json!({
        "ok": true,
        "action": "stake",
        "token": "swETH",
        "amount_eth": args.amount,
        "amount_wei": amount_wei.to_string(),
        "from": wallet,
        "contract": config::SWETH_ADDRESS,
        "txHash": tx_hash,
        "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
