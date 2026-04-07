// src/commands/redeem.rs — Redeem mTokens to get back underlying
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::find_market;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};

pub async fn run(
    chain_id: u64,
    asset: String,
    mtoken_amount: f64,
    from: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    let market = find_market(chain_id, &asset)?;
    let from_addr = match &from {
        Some(f) => f.clone(),
        None => resolve_wallet(chain_id, dry_run)?,
    };

    // mToken has 8 decimals
    let mtoken_raw = (mtoken_amount * 1e8).round() as u128;

    eprintln!(
        "[moonwell] redeem {} m{} (raw: {}) on chain {}",
        mtoken_amount, market.symbol, mtoken_raw, chain_id
    );

    // redeem(uint256) — selector 0xdb006a75
    let amount_hex = format!("{:064x}", mtoken_raw);
    let calldata = format!("0xdb006a75{}", amount_hex);
    let result = wallet_contract_call(
        chain_id,
        market.mtoken,
        &calldata,
        Some(&from_addr),
        None,
        dry_run,
    ).await?;
    let tx_hash = extract_tx_hash(&result);

    Ok(json!({
        "ok": true,
        "action": "redeem",
        "asset": market.symbol,
        "mtoken_amount": mtoken_amount,
        "mtoken_raw": mtoken_raw.to_string(),
        "mtoken": market.mtoken,
        "chain_id": chain_id,
        "dry_run": dry_run,
        "txHash": tx_hash,
        "note": format!("Redeemed {} m{} tokens for underlying {}.", mtoken_amount, market.symbol, market.symbol)
    }))
}
