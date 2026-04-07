// src/commands/supply.rs — Supply (mint mTokens)
use anyhow::Result;
use serde_json::{json, Value};
use std::time::Duration;

use crate::config::{find_market, to_raw};
use crate::onchainos::{resolve_wallet, wallet_contract_call, erc20_approve, extract_tx_hash};

pub async fn run(
    chain_id: u64,
    asset: String,
    amount: f64,
    from: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    let market = find_market(chain_id, &asset)?;
    let from_addr = match &from {
        Some(f) => f.clone(),
        None => resolve_wallet(chain_id, dry_run)?,
    };

    let amount_raw = to_raw(amount, market.underlying_decimals);

    eprintln!(
        "[moonwell] supply {} {} (raw: {}) on chain {} via mToken {}",
        amount, market.symbol, amount_raw, chain_id, market.mtoken
    );

    // Step 1: ERC20 approve
    eprintln!("[moonwell] Step 1/2: approve {} to spend {} {} ...", market.mtoken, amount, market.symbol);
    let approve_result = erc20_approve(
        chain_id,
        market.underlying,
        market.mtoken,
        amount_raw,
        Some(&from_addr),
        dry_run,
    ).await?;
    let approve_hash = extract_tx_hash(&approve_result);
    eprintln!("[moonwell] approve txHash: {}", approve_hash);

    // Step 2: Wait for nonce safety (skip in dry-run)
    if !dry_run {
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    // Step 3: mint(uint256) — selector 0xa0712d68
    eprintln!("[moonwell] Step 2/2: mToken.mint({}) ...", amount_raw);
    let amount_hex = format!("{:064x}", amount_raw);
    let mint_calldata = format!("0xa0712d68{}", amount_hex);
    let mint_result = wallet_contract_call(
        chain_id,
        market.mtoken,
        &mint_calldata,
        Some(&from_addr),
        None,
        dry_run,
    ).await?;
    let mint_hash = extract_tx_hash(&mint_result);

    Ok(json!({
        "ok": true,
        "action": "supply",
        "asset": market.symbol,
        "amount": amount,
        "amount_raw": amount_raw.to_string(),
        "mtoken": market.mtoken,
        "chain_id": chain_id,
        "dry_run": dry_run,
        "approve_txHash": approve_hash,
        "mint_txHash": mint_hash,
        "note": format!("Supplied {} {} to Moonwell. You received m{} tokens.", amount, market.symbol, market.symbol)
    }))
}
