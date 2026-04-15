use clap::Args;
use crate::calldata::{build_request_withdraw_calldata, build_claim_withdraw_calldata};
use crate::config::{
    build_approve_calldata, eeth_address, format_units, liquidity_pool_address,
    parse_units, rpc_url, withdraw_request_nft_address, CHAIN_ID,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wait_for_tx, wallet_contract_call};
use crate::rpc::{get_allowance, get_balance, is_withdrawal_finalized};

#[derive(Args)]
pub struct UnstakeArgs {
    /// Amount of eETH to withdraw (required for Step 1: request withdrawal)
    #[arg(long)]
    pub amount: Option<String>,

    /// Step 2: claim a finalized withdrawal. Requires --token-id.
    #[arg(long)]
    pub claim: bool,

    /// WithdrawRequestNFT token ID to claim (used with --claim)
    #[arg(long)]
    pub token_id: Option<u64>,

    /// Dry run — build calldata but do not broadcast
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and broadcast the transaction. Without this flag, prints a preview only.
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: UnstakeArgs) -> anyhow::Result<()> {
    if args.claim {
        run_claim(args).await
    } else {
        run_request(args).await
    }
}

/// Step 1 — approve + LiquidityPool.requestWithdraw(address recipient, uint256 amountOfEEth)
///
/// eETH uses a standard ERC-20 allowance check in requestWithdraw — LiquidityPool must be
/// approved to spend eETH before the withdrawal request can be submitted.
/// After finalization (typically a few days), use `etherfi unstake --claim --token-id <id>`.
async fn run_request(args: UnstakeArgs) -> anyhow::Result<()> {
    let amount_str = args
        .amount
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--amount is required for withdrawal request. Use --claim --token-id <id> to claim a finalized withdrawal."))?;

    let rpc = rpc_url();
    let eeth = eeth_address();
    let pool = liquidity_pool_address();

    // Parse eETH amount to wei (18 decimals)
    let eeth_wei = parse_units(amount_str, 18)?;
    if eeth_wei == 0 {
        anyhow::bail!("Amount must be greater than zero.");
    }

    // Resolve wallet address
    let wallet = resolve_wallet(CHAIN_ID)?;

    println!(
        "Requesting withdrawal of {} eETH ({} wei) via LiquidityPool.requestWithdraw()",
        amount_str, eeth_wei
    );
    println!("  eETH contract:  {}", eeth);
    println!("  LiquidityPool:  {}", pool);
    println!("  Recipient:      {}", wallet);
    println!("  Run with --confirm to broadcast.");

    // Step 1: Check eETH balance
    if !args.dry_run {
        let eeth_balance = get_balance(eeth, &wallet, rpc).await?;
        if eeth_balance < eeth_wei {
            anyhow::bail!(
                "Insufficient eETH balance. Have {} wei ({} eETH), need {} wei ({} eETH).",
                eeth_balance,
                format_units(eeth_balance, 18),
                eeth_wei,
                amount_str,
            );
        }
    }

    // Step 2: Approve LiquidityPool to spend eETH (ERC-20 allowance required by requestWithdraw)
    if !args.dry_run {
        let allowance = get_allowance(eeth, &wallet, pool, rpc).await?;
        if allowance < eeth_wei {
            println!(
                "WARNING: Approving LiquidityPool to spend eETH (unlimited allowance, u128::MAX). \
                To revoke later, call approve(LiquidityPool, 0)."
            );
            let approve_data = build_approve_calldata(pool, u128::MAX);
            let approve_result = wallet_contract_call(
                CHAIN_ID,
                eeth,
                &approve_data,
                0,
                args.confirm,
                false,
            )
            .await?;

            if approve_result["preview"].as_bool() == Some(true) {
                println!("Preview (approve): {}", serde_json::to_string_pretty(&approve_result)?);
                println!("Re-run with --confirm to execute approve + requestWithdraw.");
                return Ok(());
            }

            let approve_tx = extract_tx_hash(&approve_result).to_string();
            println!("Approve tx: {} — waiting for confirmation...", approve_tx);
            wait_for_tx(approve_tx, wallet.clone()).await
                .map_err(|e| anyhow::anyhow!("Approve tx did not confirm: {}", e))?;
            println!("Approve confirmed.");
        }
    }

    // Step 3: Call LiquidityPool.requestWithdraw(recipient, amountOfEEth)
    let calldata = build_request_withdraw_calldata(&wallet, eeth_wei);

    let result = wallet_contract_call(
        CHAIN_ID,
        pool,
        &calldata,
        0, // no ETH value — eETH is pulled via allowance
        args.confirm,
        args.dry_run,
    )
    .await?;

    if result["preview"].as_bool() == Some(true) {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result);

    // Fetch updated eETH balance after request
    let eeth_balance_str = if !args.dry_run && args.confirm {
        match get_balance(eeth, &wallet, rpc).await {
            Ok(bal) => format_units(bal, 18),
            Err(_) => "N/A".to_string(),
        }
    } else {
        "N/A".to_string()
    };

    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"action\":\"unstake_request\",\"eETHUnstaked\":\"{}\",\"eETHWei\":\"{}\",\"eETHBalance\":\"{}\",\"note\":\"Find your WithdrawRequestNFT token ID in the tx receipt, then run: etherfi unstake --claim --token-id <id> --confirm\"}}",
        tx_hash, amount_str, eeth_wei, eeth_balance_str
    );

    Ok(())
}

/// Step 2 — WithdrawRequestNFT.claimWithdraw(uint256 tokenId)
///
/// Burns the WithdrawRequestNFT and sends ETH to the original recipient.
/// Only callable after the withdrawal request is finalized (isFinalized returns true).
async fn run_claim(args: UnstakeArgs) -> anyhow::Result<()> {
    let token_id = args
        .token_id
        .ok_or_else(|| anyhow::anyhow!("--token-id is required when using --claim."))?;

    let rpc = rpc_url();
    let nft = withdraw_request_nft_address();

    // Resolve wallet address
    let wallet = resolve_wallet(CHAIN_ID)?;

    // Check finalization status
    let finalized = if !args.dry_run {
        is_withdrawal_finalized(nft, token_id, rpc).await.unwrap_or(false)
    } else {
        true
    };

    if !finalized && !args.dry_run {
        eprintln!(
            "Warning: WithdrawRequestNFT #{} is not yet finalized. \
            Claiming before finalization will fail on-chain. \
            Check the ether.fi UI or try again later.",
            token_id
        );
        if args.confirm {
            anyhow::bail!(
                "Withdrawal request #{} is not finalized. Cannot claim yet.",
                token_id
            );
        }
    }

    println!(
        "Claiming withdrawal for WithdrawRequestNFT #{} via WithdrawRequestNFT.claimWithdraw()",
        token_id
    );
    println!("  WithdrawRequestNFT: {}", nft);
    println!("  Wallet: {}", wallet);
    println!("  Finalized: {}", finalized);
    println!("  Run with --confirm to broadcast.");

    let calldata = build_claim_withdraw_calldata(token_id);

    let result = wallet_contract_call(
        CHAIN_ID,
        nft,
        &calldata,
        0,
        args.confirm,
        args.dry_run,
    )
    .await?;

    if result["preview"].as_bool() == Some(true) {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let tx_hash = extract_tx_hash(&result);

    println!(
        "{{\"ok\":true,\"txHash\":\"{}\",\"action\":\"unstake_claim\",\"tokenId\":{},\"finalized\":{}}}",
        tx_hash, token_id, finalized
    );

    Ok(())
}
