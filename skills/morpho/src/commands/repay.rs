use anyhow::Context;
use crate::api;
use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Repay Morpho Blue debt.
/// If `amount` is Some, does a partial repay by assets.
/// If `all` is true, repays all debt using borrow shares from the GraphQL API.
pub async fn run(
    market_id: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
    confirm: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let borrower_string = onchainos::resolve_wallet(from, chain_id).await?;
    let borrower = borrower_string.as_str();

    // Fetch market params from GraphQL API
    let market = api::get_market(market_id, chain_id).await
        .context("Failed to fetch market from Morpho API")?;
    let mp = api::build_market_params(&market)?;

    let loan_token = mp.loan_token.clone();
    let decimals = rpc::erc20_decimals(&loan_token, cfg.rpc_url).await.unwrap_or(18);
    let symbol = rpc::erc20_symbol(&loan_token, cfg.rpc_url).await.unwrap_or_else(|_| "TOKEN".to_string());

    let repay_assets: u128;
    let repay_shares: u128;
    let display_amount: String;
    let borrow_assets: u128;

    if all {
        if dry_run {
            // In dry-run mode, skip live position check — no debt to look up
            repay_shares = 0;
            repay_assets = 0;
            borrow_assets = 0;
            display_amount = "0 (dry-run)".to_string();
            eprintln!("[morpho] [dry-run] Repaying all debt (skipping live position check)...");
        } else {
        // Fetch borrow shares for full repayment via GraphQL positions
        let positions = api::get_user_positions(borrower, chain_id).await?;
        let pos = positions.iter().find(|p| p.market.unique_key == market_id)
            .context("No position found for this market. Nothing to repay.")?;

        let borrow_shares_str = pos.state.borrow_shares.as_deref().unwrap_or("0");
        repay_shares = borrow_shares_str.parse().unwrap_or(0);
        repay_assets = 0; // Use shares mode for full repay

        let borrow_assets_str = pos.state.borrow_assets.as_deref().unwrap_or("0");
        borrow_assets = borrow_assets_str.parse().unwrap_or(0);
        display_amount = calldata::format_amount(borrow_assets, decimals);

        eprintln!("[morpho] Repaying all debt ({} {}) using {} shares...", display_amount, symbol, repay_shares);
        }
    } else {
        let amt_str = amount.context("Must provide --amount or --all")?;
        repay_assets = calldata::parse_amount(amt_str, decimals)?;
        repay_shares = 0;
        borrow_assets = 0;
        display_amount = amt_str.to_string();
        eprintln!("[morpho] Repaying {} {} to Morpho Blue market {}...", amt_str, symbol, market_id);
    }

    // Confirm gate: show preview and exit if --confirm not given
    if !dry_run && !confirm {
        let preview = serde_json::json!({
            "ok": true,
            "preview": true,
            "operation": "repay",
            "marketId": market_id,
            "loanAsset": symbol,
            "loanAssetAddress": loan_token,
            "amount": display_amount,
            "repayAll": all,
            "chainId": chain_id,
            "morphoBlue": cfg.morpho_blue,
            "pendingTransactions": 2,
            "transactions": [
                {"step": 1, "description": format!("Approve Morpho Blue to spend {} {}", display_amount, symbol), "to": loan_token},
                {"step": 2, "description": format!("Repay {} {} to Morpho Blue market {}", display_amount, symbol, market_id), "to": cfg.morpho_blue},
            ],
            "note": "Re-run with --confirm to execute these transactions on-chain."
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // Step 1: Approve Morpho Blue to spend loan token
    // Add a small buffer (0.5%) to the approval amount to cover accrued interest
    let approve_amount = if all && repay_assets == 0 {
        // For full repay via shares, approve actual borrow amount + 1% buffer for accrued interest
        borrow_assets + borrow_assets / 100
    } else {
        repay_assets + repay_assets / 200 // +0.5% buffer
    };

    let approve_calldata = calldata::encode_approve(cfg.morpho_blue, approve_amount);
    eprintln!("[morpho] Step 1/2: Approving Morpho Blue to spend {}...", symbol);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, loan_token, approve_calldata);
    }
    let approve_result = onchainos::wallet_contract_call(chain_id, &loan_token, &approve_calldata, from, None, dry_run, true).await?;  // --force: approval is a prerequisite step
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    onchainos::wait_for_tx(&approve_tx, cfg.rpc_url, chain_id).await?;

    // Step 2: repay(marketParams, assets, shares, onBehalf, data)
    let repay_calldata = calldata::encode_repay(&mp, repay_assets, repay_shares, borrower);

    eprintln!("[morpho] Step 2/2: Repaying debt...");
    if dry_run {
        eprintln!("[morpho] [dry-run] Would call: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, cfg.morpho_blue, repay_calldata);
    }

    // After user confirmation, submit the repay transaction
    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.morpho_blue,
        &repay_calldata,
        from,
        None,
        dry_run,
        false,
    ).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "repay",
        "marketId": market_id,
        "loanAsset": symbol,
        "loanAssetAddress": loan_token,
        "amount": display_amount,
        "repayAll": all,
        "chainId": chain_id,
        "morphoBlue": cfg.morpho_blue,
        "dryRun": dry_run,
        "approveTxHash": approve_tx,
        "repayTxHash": tx_hash,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
