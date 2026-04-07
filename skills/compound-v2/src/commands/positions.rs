// src/commands/positions.rs — Show user's supplied and borrowed positions
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{MARKETS, RPC_URL};
use crate::onchainos::resolve_wallet;
use crate::rpc::{balance_of, borrow_balance_current, exchange_rate_current, ctoken_to_underlying};

pub async fn run(chain_id: u64, wallet: Option<String>) -> Result<Value> {
    if chain_id != 1 {
        anyhow::bail!("Compound V2 is only supported on Ethereum mainnet (chain 1). Got chain {}.", chain_id);
    }

    let address = match wallet {
        Some(w) => w,
        None => resolve_wallet(chain_id)?,
    };

    let rpc = RPC_URL;
    let mut positions = Vec::new();

    for m in MARKETS {
        let ctoken_bal = balance_of(m.ctoken, &address, rpc).await.unwrap_or(0);
        let borrow_bal = borrow_balance_current(m.ctoken, &address, rpc).await.unwrap_or(0);
        let exchange_rate = exchange_rate_current(m.ctoken, rpc).await.unwrap_or(0);

        // Compute underlying supplied
        let underlying_raw = ctoken_to_underlying(ctoken_bal, exchange_rate);
        let underlying_human = underlying_raw / 10f64.powi(m.underlying_decimals as i32);
        let borrow_human = (borrow_bal as f64) / 10f64.powi(m.underlying_decimals as i32);
        let ctoken_human = (ctoken_bal as f64) / 10f64.powi(m.ctoken_decimals as i32);

        if ctoken_bal > 0 || borrow_bal > 0 {
            positions.push(json!({
                "asset": m.symbol,
                "ctoken_address": m.ctoken,
                "ctoken_balance": format!("{:.8}", ctoken_human),
                "supplied_underlying": format!("{:.8}", underlying_human),
                "borrowed": format!("{:.8}", borrow_human)
            }));
        }
    }

    if positions.is_empty() {
        return Ok(json!({
            "ok": true,
            "chain_id": chain_id,
            "wallet": address,
            "positions": [],
            "message": "No active positions found on Compound V2."
        }));
    }

    Ok(json!({
        "ok": true,
        "chain_id": chain_id,
        "wallet": address,
        "positions": positions
    }))
}
