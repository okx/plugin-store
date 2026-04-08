// src/commands/positions.rs — View user's mToken positions
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{chain_config};
use crate::onchainos::resolve_wallet;
use crate::rpc::{balance_of, borrow_balance_current, exchange_rate_current, mtoken_to_underlying_raw};

pub async fn run(chain_id: u64, wallet: Option<String>) -> Result<Value> {
    let cfg = chain_config(chain_id)?;
    let rpc = cfg.rpc;

    let wallet_addr = match wallet {
        Some(w) => w,
        None => resolve_wallet(chain_id, false)?,
    };

    let mut positions = Vec::new();

    for m in cfg.markets {
        let mtoken_bal = balance_of(m.mtoken, &wallet_addr, rpc).await.unwrap_or(0);
        let borrow_bal = borrow_balance_current(m.mtoken, &wallet_addr, rpc).await.unwrap_or(0);

        if mtoken_bal == 0 && borrow_bal == 0 {
            continue;
        }

        let exchange_rate = exchange_rate_current(m.mtoken, rpc).await.unwrap_or(0);
        let underlying_raw = mtoken_to_underlying_raw(mtoken_bal, exchange_rate);

        let mtoken_dec = 10f64.powi(m.mtoken_decimals as i32);
        let underlying_dec = 10f64.powi(m.underlying_decimals as i32);

        let supplied_human = underlying_raw / underlying_dec;
        let borrowed_human = (borrow_bal as f64) / underlying_dec;
        let mtoken_human = (mtoken_bal as f64) / mtoken_dec;

        positions.push(json!({
            "symbol": m.symbol,
            "mtoken": m.mtoken,
            "mtoken_balance": format!("{:.8}", mtoken_human),
            "supplied": format!("{:.6}", supplied_human),
            "borrowed": format!("{:.6}", borrowed_human),
        }));
    }

    Ok(json!({
        "ok": true,
        "chain_id": chain_id,
        "wallet": wallet_addr,
        "protocol": "Moonwell Flagship",
        "positions": positions,
        "note": if positions.is_empty() { "No active positions found" } else { "Active positions shown" }
    }))
}
