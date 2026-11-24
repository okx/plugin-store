use clap::Args;
use crate::calldata::{build_request_withdraw_calldata, build_claim_withdraw_calldata};
use crate::config::{
    build_approve_calldata, eeth_address, format_units, liquidity_pool_address,
    parse_units, rpc_url, withdraw_request_nft_address, CHAIN_ID,
};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wait_for_tx, wallet_contract_call};
use crate::rpc::{get_allowance, get_balance, get_nft_token_id_from_mint, is_withdrawal_finalized};

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

    eprintln!("Requesting withdrawal of {} eETH ({} wei) via LiquidityPool.requestWithdraw()", amount_str, eeth_wei);
    eprintln!("  eETH contract:  {}", eeth);
    eprintln!("  LiquidityPool:  {}", pool);
    eprintln!("  Recipient:      {}", wallet);
    eprintln!("  Run with --confirm to broadcast.");

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
            eprintln!(
                "Approving LiquidityPool to spend exactly {} wei of eETH.",
                eeth_wei
            );
            let approve_data = build_approve_calldata(pool, eeth_wei);
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
                eprintln!("NOTE: eETH approval needed. Re-run with --confirm to approve + requestWithdraw.");
                println!("{}", serde_json::to_string_pretty(&approve_result)?);
                return Ok(());
            }

            // Only reached when --confirm is passed and tx is actually broadcast
            let approve_tx = extract_tx_hash(&approve_result).to_string();
            eprintln!("Approve tx: {} — waiting for confirmation...", approve_tx);
            wait_for_tx(approve_tx, wallet.clone()).await
                .map_err(|e| anyhow::anyhow!("Approve tx did not confirm: {}", e))?;
            eprintln!("Approve confirmed.");
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

    // Wait for requestWithdraw tx to confirm before querying balance / receipt
    // (fix: was querying before confirmation, showing stale pre-tx balance)
    if !args.dry_run && args.confirm {
        eprintln!("RequestWithdraw tx: {} — waiting for confirmation...", tx_hash);
        wait_for_tx(tx_hash.to_string(), wallet.clone()).await
            .map_err(|e| anyhow::anyhow!("RequestWithdraw tx did not confirm: {}", e))?;
        eprintln!("RequestWithdraw confirmed.");
    }

    // Fetch updated eETH balance after confirmation
    let eeth_balance_str = if !args.dry_run && args.confirm {
        match get_balance(eeth, &wallet, rpc).await {
            Ok(bal) => format_units(bal, 18),
            Err(_) => "N/A".to_string(),
        }
    } else {
        "N/A".to_string()
    };

    // Extract NFT token ID from tx receipt
    let nft = withdraw_request_nft_address();
    let nft_token_id: Option<u64> = if !args.dry_run && args.confirm {
        get_nft_token_id_from_mint(tx_hash, nft, &wallet, rpc).await.unwrap_or(None)
    } else {
        None
    };

    let note = match nft_token_id {
        Some(id) => format!(
            "WithdrawRequestNFT #{id} minted. Withdrawals typically take 1-7 days. \
            Track at https://app.ether.fi/portfolio — \
            then run: etherfi unstake --claim --token-id {id} --confirm"
        ),
        None => "Find your WithdrawRequestNFT token ID in the tx receipt, \
            then run: etherfi unstake --claim --token-id <id> --confirm. \
            Withdrawals typically take 1-7 days; track at https://app.ether.fi/portfolio".to_string(),
    };

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "txHash": tx_hash,
            "action": "unstake_request",
            "eETHUnstaked": amount_str,
            "eETHWei": eeth_wei.to_string(),
            "eETHBalance": eeth_balance_str,
            "nftTokenId": nft_token_id,
            "note": note,
        })
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
            Withdrawals typically take 1-7 days depending on the exit queue. \
            Track your request at https://app.ether.fi/portfolio — \
            run `etherfi unstake --claim --token-id {} --confirm` once finalized.",
            token_id, token_id
        );
        if args.confirm {
            anyhow::bail!(
                "Withdrawal request #{} is not finalized. Cannot claim yet. \
                Typically takes 1-7 days — check https://app.ether.fi/portfolio for status.",
                token_id
            );
        }
    }

    eprintln!("Claiming withdrawal for WithdrawRequestNFT #{} via WithdrawRequestNFT.claimWithdraw()", token_id);
    eprintln!("  WithdrawRequestNFT: {}", nft);
    eprintln!("  Wallet: {}", wallet);
    eprintln!("  Finalized: {}", finalized);
    eprintln!("  Run with --confirm to broadcast.");

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
