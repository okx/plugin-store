// src/commands/borrow.rs — Borrow (DRY-RUN ONLY)
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{find_market, to_raw};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash};

pub async fn run(
    chain_id: u64,
    asset: String,
    amount: f64,
    from: Option<String>,
    dry_run: bool,
) -> Result<Value> {
    if !dry_run {
        anyhow::bail!(
            "borrow is only available in --dry-run mode for safety. \
             Borrowing requires sufficient collateral and carries liquidation risk."
        );
    }

    let market = find_market(chain_id, &asset)?;
    let from_addr = match &from {
        Some(f) => f.clone(),
        None => resolve_wallet(chain_id, dry_run)?,
    };

    let amount_raw = to_raw(amount, market.underlying_decimals);

    eprintln!(
        "[moonwell] borrow (dry-run) {} {} (raw: {}) on chain {}",
        amount, market.symbol, amount_raw, chain_id
    );

    // borrow(uint256) — selector 0xc5ebeaec
    let amount_hex = format!("{:064x}", amount_raw);
    let calldata = format!("0xc5ebeaec{}", amount_hex);
    let result = wallet_contract_call(
        chain_id,
        market.mtoken,
        &calldata,
        Some(&from_addr),
        None,
        true, // always dry_run for borrow
    ).await?;
    let tx_hash = extract_tx_hash(&result);

    Ok(json!({
        "ok": true,
        "action": "borrow",
        "asset": market.symbol,
        "amount": amount,
        "amount_raw": amount_raw.to_string(),
        "mtoken": market.mtoken,
        "chain_id": chain_id,
        "dry_run": true,
        "txHash": tx_hash,
        "warning": "DRY-RUN ONLY. Real borrow requires collateral and carries liquidation risk.",
        "calldata": format!("0xc5ebeaec{}", amount_hex)
    }))
}
