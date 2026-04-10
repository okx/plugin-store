use anyhow::Context;
use crate::api;
use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Withdraw collateral from a Morpho Blue market.
pub async fn run(
    market_id: &str,
    amount: Option<&str>,
    all: bool,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let owner_string = onchainos::resolve_wallet(from, chain_id).await?;
    let owner = owner_string.as_str();

    // Fetch market params from GraphQL API
    let market = api::get_market(market_id, chain_id).await
        .context("Failed to fetch market from Morpho API")?;
    let mp = api::build_market_params(&market)?;

    let collateral_token = mp.collateral_token.clone();
    let decimals = rpc::erc20_decimals(&collateral_token, cfg.rpc_url).await.unwrap_or(18);
    let symbol = rpc::erc20_symbol(&collateral_token, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| "TOKEN".to_string());

    let raw_amount: u128;
    let display_amount: String;

    if all {
        // Fetch collateral balance from GraphQL positions
        let positions = api::get_user_positions(owner, chain_id).await?;
        let pos = positions.iter().find(|p| p.market.unique_key == market_id)
            .context("No position found for this market. Nothing to withdraw.")?;

        let collateral_str = pos.state.collateral.as_deref().unwrap_or("0");
        raw_amount = collateral_str.parse().unwrap_or(0);
        display_amount = calldata::format_amount(raw_amount, decimals);
        eprintln!("[morpho] Withdrawing all collateral ({} {}) from market {}...", display_amount, symbol, market_id);
    } else {
        let amt_str = amount.context("Must provide --amount or --all")?;
        raw_amount = calldata::parse_amount(amt_str, decimals)?;
        display_amount = amt_str.to_string();
        eprintln!("[morpho] Withdrawing {} {} collateral from market {}...", amt_str, symbol, market_id);
    }

    // withdrawCollateral(marketParams, assets, onBehalf, receiver)
    let withdraw_calldata = calldata::encode_withdraw_collateral(&mp, raw_amount, owner, owner);

    if dry_run {
        eprintln!("[morpho] [dry-run] Would call: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, cfg.morpho_blue, withdraw_calldata);
    }

    // Ask user to confirm before executing on-chain
    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.morpho_blue,
        &withdraw_calldata,
        from,
        None,
        dry_run,
        false,
    ).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "withdraw-collateral",
        "marketId": market_id,
        "collateralAsset": symbol,
        "collateralAssetAddress": collateral_token,
        "amount": display_amount,
        "rawAmount": raw_amount.to_string(),
        "chainId": chain_id,
        "morphoBlue": cfg.morpho_blue,
        "dryRun": dry_run,
        "txHash": tx_hash,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
