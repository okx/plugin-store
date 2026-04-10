use anyhow::Context;
use crate::api;
use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Supply collateral to a Morpho Blue market.
pub async fn run(
    market_id: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<()> {
    let cfg = get_chain_config(chain_id)?;
    let supplier_string = onchainos::resolve_wallet(from, chain_id).await?;
    let supplier = supplier_string.as_str();

    // Fetch market params from GraphQL API
    let market = api::get_market(market_id, chain_id).await
        .context("Failed to fetch market from Morpho API")?;
    let mp = api::build_market_params(&market)?;

    let collateral_token = mp.collateral_token.clone();
    let decimals = rpc::erc20_decimals(&collateral_token, cfg.rpc_url).await.unwrap_or(18);
    let symbol = rpc::erc20_symbol(&collateral_token, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| "TOKEN".to_string());

    let raw_amount = calldata::parse_amount(amount, decimals)?;

    // Step 1: Approve Morpho Blue to spend collateral token (ask user to confirm before executing)
    let approve_calldata = calldata::encode_approve(cfg.morpho_blue, raw_amount);
    eprintln!("[morpho] Step 1/2: Approving Morpho Blue to spend {} {}...", amount, symbol);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would approve: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, collateral_token, approve_calldata);
    }
    let approve_result = onchainos::wallet_contract_call(chain_id, &collateral_token, &approve_calldata, from, None, dry_run, true).await?;  // --force: approval is a prerequisite step
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    onchainos::wait_for_tx(&approve_tx, cfg.rpc_url, chain_id).await?;

    // Step 2: supplyCollateral(marketParams, assets, onBehalf, data)
    let supply_calldata = calldata::encode_supply_collateral(&mp, raw_amount, supplier);
    eprintln!("[morpho] Step 2/2: Supplying {} {} as collateral to market {}...", amount, symbol, market_id);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would call: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, cfg.morpho_blue, supply_calldata);
    }

    // After user confirmation, submit the supply collateral transaction
    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.morpho_blue,
        &supply_calldata,
        from,
        None,
        dry_run,
        false,
    ).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "supply-collateral",
        "marketId": market_id,
        "collateralAsset": symbol,
        "collateralAssetAddress": collateral_token,
        "amount": amount,
        "rawAmount": raw_amount.to_string(),
        "chainId": chain_id,
        "morphoBlue": cfg.morpho_blue,
        "dryRun": dry_run,
        "approveTxHash": approve_tx,
        "supplyCollateralTxHash": tx_hash,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
