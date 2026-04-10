use anyhow::Context;
use crate::api;
use crate::calldata;
use crate::config::get_chain_config;
use crate::onchainos;
use crate::rpc;

/// Borrow from a Morpho Blue market.
pub async fn run(
    market_id: &str,
    amount: &str,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
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

    let raw_amount = calldata::parse_amount(amount, decimals)?;

    // borrow(marketParams, assets, 0, onBehalf, receiver)
    let borrow_calldata = calldata::encode_borrow(&mp, raw_amount, 0, borrower, borrower);

    eprintln!("[morpho] Borrowing {} {} from Morpho Blue market {}...", amount, symbol, market_id);
    if dry_run {
        eprintln!("[morpho] [dry-run] Would call: onchainos wallet contract-call --chain {} --to {} --input-data {}", chain_id, cfg.morpho_blue, borrow_calldata);
    }

    // Ask user to confirm before executing on-chain
    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.morpho_blue,
        &borrow_calldata,
        from,
        None,
        dry_run,
        false,
    ).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "operation": "borrow",
        "marketId": market_id,
        "loanAsset": symbol,
        "loanAssetAddress": loan_token,
        "amount": amount,
        "rawAmount": raw_amount.to_string(),
        "chainId": chain_id,
        "morphoBlue": cfg.morpho_blue,
        "dryRun": dry_run,
        "txHash": tx_hash,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
