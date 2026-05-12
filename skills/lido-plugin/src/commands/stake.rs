use crate::{config, onchainos, rpc};
use clap::Args;

#[derive(Args)]
pub struct StakeArgs {
    /// Amount of ETH to stake (in ETH, not wei). Example: 1.5
    #[arg(long)]
    pub amount_eth: f64,

    /// Referral address (optional, defaults to zero address)
    #[arg(long)]
    pub referral: Option<String>,

    /// Wallet address to stake from (optional, resolved from onchainos if omitted)
    #[arg(long)]
    pub from: Option<String>,

    /// Dry run — show calldata without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: StakeArgs) -> anyhow::Result<()> {
    let chain_id = config::CHAIN_ID;

    // Resolve wallet address
    let wallet = match args.from.clone() {
        Some(f) => f,
        None => onchainos::resolve_wallet(chain_id).await.unwrap_or_default(),
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot get wallet address. Pass --from or ensure onchainos is logged in.");
    }

    // Convert ETH to wei — round() avoids f64 truncation on values like 0.1 ETH
    let amount_wei = (args.amount_eth * 1e18_f64).round() as u128;
    if amount_wei == 0 {
        anyhow::bail!("Stake amount must be greater than 0");
    }

    // Pre-flight: check isStakingPaused()
    // EVM-012: this is a safety gate — silent unwrap_or(0) would treat decode
    // failure as "not paused" and let the user stake into a paused contract.
    // Bail explicitly so the caller sees the failure mode.
    let paused_calldata = format!("0x{}", config::SEL_IS_STAKING_PAUSED);
    let paused_result = onchainos::eth_call(chain_id, config::STETH_ADDRESS, &paused_calldata).await?;
    let return_data = rpc::extract_return_data(&paused_result)
        .map_err(|e| anyhow::anyhow!("Failed to read isStakingPaused() return data: {:#}", e))?;
    let val = rpc::decode_uint256(&return_data)
        .map_err(|e| anyhow::anyhow!("Failed to decode isStakingPaused() result: {:#}", e))?;
    if val != 0 {
        anyhow::bail!("Lido staking is currently paused. Please try again later.");
    }

    // Referral address (zero address if not specified)
    let referral = args
        .referral
        .as_deref()
        .unwrap_or("0x0000000000000000000000000000000000000000");
    let referral_padded = rpc::encode_address(referral);

    // Build calldata: submit(address _referral)
    let calldata = format!("0x{}{}", config::SEL_SUBMIT, referral_padded);

    if args.dry_run {
        println!("{}", serde_json::json!({
            "ok": true,
            "dry_run": true,
            "action": "stake",
            "from": wallet,
            "amountEth": format!("{:.6}", args.amount_eth),
            "amountWei": amount_wei.to_string(),
            "referral": referral,
            "contract": config::STETH_ADDRESS,
            "calldata": calldata,
            "note": "Add --confirm to broadcast"
        }));
        return Ok(());
    }

    // Pre-flight: ETH balance check (EVM-001)
    let eth_balance = onchainos::eth_get_balance(&wallet, chain_id).await
        .map_err(|e| anyhow::anyhow!("Failed to check ETH balance: {}", e))?;
    if eth_balance < amount_wei {
        anyhow::bail!(
            "Insufficient ETH balance: need {:.6} ETH, have {:.6} ETH.",
            amount_wei as f64 / 1e18,
            eth_balance as f64 / 1e18
        );
    }

    if !args.confirm {
        println!("{}", serde_json::json!({
            "ok": true,
            "preview": true,
            "action": "stake",
            "from": wallet,
            "amountEth": format!("{:.6}", args.amount_eth),
            "amountWei": amount_wei.to_string(),
            "referral": referral,
            "stEthExpected": format!("~{:.6}", args.amount_eth),
            "note": "Add --confirm to execute"
        }));
        return Ok(());
    }

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::STETH_ADDRESS,
        &calldata,
        Some(&wallet),
        Some(amount_wei),
        args.confirm,
        args.dry_run,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!("{}", serde_json::json!({
        "ok": true,
        "action": "stake",
        "from": wallet,
        "amountEth": format!("{:.6}", args.amount_eth),
        "amountWei": amount_wei.to_string(),
        "txHash": tx_hash,
        "stEthExpected": format!("~{:.6}", args.amount_eth),
        "note": "stETH balance grows daily via rebase"
    }));

    Ok(())
}
