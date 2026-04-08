// src/commands/repay.rs — Repay borrow (DRY-RUN ONLY)
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{find_market, to_raw};
use crate::onchainos::{resolve_wallet, wallet_contract_call, erc20_approve, extract_tx_hash};

pub async fn run(
    chain_id: u64,
    asset: String,
    amount: f64,
    from: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    if !dry_run {
        anyhow::bail!(
            "repay is only available in --dry-run mode for safety. \
             Use --dry-run to preview the transaction."
        );
    }

    let market = find_market(chain_id, &asset)?;
    let from_addr = match &from {
        Some(f) => f.clone(),
        None => resolve_wallet(chain_id, dry_run)?,
    };

    let amount_raw = to_raw(amount, market.underlying_decimals);

    eprintln!(
        "[moonwell] repay (dry-run) {} {} (raw: {}) on chain {}",
        amount, market.symbol, amount_raw, chain_id
    );

    // Step 1: approve
    let approve_result = erc20_approve(
        chain_id,
        market.underlying,
        market.mtoken,
        amount_raw,
        Some(&from_addr),
        true, // always dry_run for repay
    ).await?;
    let approve_hash = extract_tx_hash(&approve_result);

    // Step 2: repayBorrow(uint256) — selector 0x0e752702
    let amount_hex = format!("{:064x}", amount_raw);
    let calldata = format!("0x0e752702{}", amount_hex);
    let result = wallet_contract_call(
        chain_id,
        market.mtoken,
        &calldata,
        Some(&from_addr),
        None,
        true, // always dry_run for repay
    ).await?;
    let tx_hash = extract_tx_hash(&result);

    Ok(json!({
        "ok": true,
        "action": "repay",
        "asset": market.symbol,
        "amount": amount,
        "amount_raw": amount_raw.to_string(),
        "mtoken": market.mtoken,
        "chain_id": chain_id,
        "dry_run": true,
        "approve_txHash": approve_hash,
        "repay_txHash": tx_hash,
        "warning": "DRY-RUN ONLY. Real repay executes on-chain.",
        "repay_calldata": format!("0x0e752702{}", amount_hex)
    }))
}
