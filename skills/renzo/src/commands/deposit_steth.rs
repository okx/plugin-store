use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct DepositStethArgs {
    /// Amount of stETH to deposit (in ETH units, not wei). Example: 0.00005
    #[arg(long)]
    pub amount: f64,

    /// Wallet address to deposit from (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: DepositStethArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    if args.amount <= 0.0 {
        anyhow::bail!("Deposit amount must be greater than 0");
    }

    let amount_wei = (args.amount * 1e18) as u128;

    // Pre-flight: check paused()
    let paused_calldata = format!("0x{}", config::SEL_PAUSED);
    if let Ok(result) = onchainos::eth_call(config::RESTAKE_MANAGER, &paused_calldata, config::RPC_URL) {
        if let Ok(raw) = rpc::extract_return_data(&result) {
            let val = rpc::decode_uint256(&raw).unwrap_or(0);
            if val != 0 {
                anyhow::bail!("Renzo RestakeManager is currently paused. Please try again later.");
            }
        }
    }

    // Build calldatas
    let approve_calldata = rpc::calldata_approve(config::RESTAKE_MANAGER, amount_wei);
    let deposit_calldata = rpc::calldata_deposit_token(config::STETH_ADDRESS, amount_wei);

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "step1_approve": {
                "calldata": approve_calldata,
                "to": config::STETH_ADDRESS,
                "description": "approve(RestakeManager, amount)"
            },
            "step2_deposit": {
                "calldata": deposit_calldata,
                "to": config::RESTAKE_MANAGER,
                "description": "deposit(stETH, amount)"
            },
            "amount": args.amount,
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

    // Check current allowance — skip approve if already sufficient
    let allowance_calldata = rpc::calldata_allowance(&wallet, config::RESTAKE_MANAGER);
    let allowance_result =
        onchainos::eth_call(config::STETH_ADDRESS, &allowance_calldata, config::RPC_URL)?;
    let allowance_raw = rpc::extract_return_data(&allowance_result)?;
    let current_allowance = rpc::decode_uint256(&allowance_raw).unwrap_or(0);

    let approve_tx_hash = if current_allowance < amount_wei {
        // Step 1: approve stETH to RestakeManager
        let approve_result = onchainos::wallet_contract_call(
            chain_id,
            config::STETH_ADDRESS,
            &approve_calldata,
            Some(&wallet),
            None,
            false,
        )
        .await?;
        let hash = onchainos::extract_tx_hash(&approve_result).to_string();
        // Small delay to allow approve to confirm before deposit
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        hash
    } else {
        "skipped_sufficient_allowance".to_string()
    };

    // Step 2: deposit stETH
    let deposit_result = onchainos::wallet_contract_call(
        chain_id,
        config::RESTAKE_MANAGER,
        &deposit_calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let deposit_tx_hash = onchainos::extract_tx_hash(&deposit_result);

    println!("{}", serde_json::json!({
        "ok": true,
        "data": {
            "approve_txHash": approve_tx_hash,
            "deposit_txHash": deposit_tx_hash,
            "from": wallet,
            "amount": args.amount,
            "amount_wei": amount_wei.to_string(),
            "description": format!("Deposited {} stETH into Renzo; ezETH minted to {}", args.amount, wallet),
            "explorer": format!("https://etherscan.io/tx/{}", deposit_tx_hash)
        }
    }));

    Ok(())
}
