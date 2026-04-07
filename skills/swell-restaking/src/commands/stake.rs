use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to restake (human-readable, e.g. "0.00005")
    #[arg(long)]
    pub amount: String,

    /// Wallet address to stake from (optional; resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Simulate without broadcasting (shows calldata only)
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs, chain_id: u64) -> anyhow::Result<()> {
    let amount_wei = rpc::parse_eth_to_wei(&args.amount)?;
    if amount_wei == 0 {
        anyhow::bail!("Stake amount must be greater than 0");
    }

    // Build calldata: deposit() — selector only, payable, no parameters
    // Selector 0xd0e30db0 verified via cast sig "deposit()"
    let calldata = rpc::calldata_noarg(config::SEL_DEPOSIT);

    // Show preview (before wallet resolution for dry-run efficiency)
    eprintln!("=== Swell Restaking: ETH -> rswETH (EigenLayer) ===");
    eprintln!("Amount:    {} ETH ({} wei)", args.amount, amount_wei);
    eprintln!("Contract:  {} (rswETH)", config::RSWETH_ADDRESS);
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

    // Pre-flight: fetch current rswETH rate to display expected output
    let eth_to_rsweth_calldata = rpc::calldata_noarg(config::SEL_ETH_TO_RSWETH_RATE);
    let rate_result = onchainos::eth_call(chain_id, config::RSWETH_ADDRESS, &eth_to_rsweth_calldata);
    if let Ok(rate_raw) = rate_result {
        if let Ok(return_data) = rpc::extract_return_data(&rate_raw) {
            if let Ok(rate) = rpc::decode_uint256(&return_data) {
                let expected_rsweth = (amount_wei as u128).saturating_mul(rate) / 1_000_000_000_000_000_000u128;
                eprintln!("Expected rswETH: ~{}", rpc::format_eth(expected_rsweth));
            }
        }
    }
    eprintln!("From:      {}", wallet);
    eprintln!();
    eprintln!("Ask user to confirm before proceeding with the transaction.");
    eprintln!();

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::RSWETH_ADDRESS,
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
        "token_received": "rswETH",
        "protocol": "Swell Restaking (EigenLayer)",
        "amount_eth": args.amount,
        "amount_wei": amount_wei.to_string(),
        "from": wallet,
        "contract": config::RSWETH_ADDRESS,
        "txHash": tx_hash,
        "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
        "note": "rswETH earns both validator rewards and EigenLayer restaking rewards"
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
