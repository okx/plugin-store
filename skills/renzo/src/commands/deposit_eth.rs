use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct DepositEthArgs {
    /// Amount of ETH to deposit (in ETH, not wei). Example: 0.00005
    #[arg(long)]
    pub amount_eth: f64,

    /// Wallet address to deposit from (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: DepositEthArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    if args.amount_eth <= 0.0 {
        anyhow::bail!("Deposit amount must be greater than 0");
    }

    let amount_wei = (args.amount_eth * 1e18) as u128;

    // Pre-flight: check paused() — must happen before dry_run early return
    // to give meaningful error even in dry-run
    let paused_calldata = format!("0x{}", config::SEL_PAUSED);
    if let Ok(result) = onchainos::eth_call(config::RESTAKE_MANAGER, &paused_calldata, config::RPC_URL) {
        if let Ok(raw) = rpc::extract_return_data(&result) {
            let val = rpc::decode_uint256(&raw).unwrap_or(0);
            if val != 0 {
                anyhow::bail!("Renzo RestakeManager is currently paused. Please try again later.");
            }
        }
    }

    // Calldata: depositETH() — no parameters
    let calldata = format!("0x{}", config::SEL_DEPOSIT_ETH);

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": calldata,
            "to": config::RESTAKE_MANAGER,
            "amount_eth": args.amount_eth,
            "amount_wei": amount_wei.to_string()
        }));
        return Ok(());
    }

    // Resolve wallet after dry_run guard
    let wallet = args
        .from
        .clone()
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::RESTAKE_MANAGER,
        &calldata,
        Some(&wallet),
        Some(amount_wei),
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    println!("{}", serde_json::json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "from": wallet,
            "amount_eth": args.amount_eth,
            "amount_wei": amount_wei.to_string(),
            "description": format!("Deposited {} ETH into Renzo; ezETH minted to {}", args.amount_eth, wallet),
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
        }
    }));

    Ok(())
}
