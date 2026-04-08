// src/commands/markets.rs — List Moonwell mToken markets with APR and exchange rates
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{chain_config, SECONDS_PER_YEAR};
use crate::rpc::{supply_rate_per_timestamp, borrow_rate_per_timestamp, exchange_rate_current, rate_to_apr_pct};

pub async fn run(chain_id: u64) -> Result<Value> {
    let cfg = chain_config(chain_id)?;
    let rpc = cfg.rpc;
    let mut markets = Vec::new();

    for m in cfg.markets {
        let supply_rate = supply_rate_per_timestamp(m.mtoken, rpc).await.unwrap_or(0);
        let borrow_rate = borrow_rate_per_timestamp(m.mtoken, rpc).await.unwrap_or(0);
        let exchange_rate = exchange_rate_current(m.mtoken, rpc).await.unwrap_or(0);

        let supply_apr = rate_to_apr_pct(supply_rate, SECONDS_PER_YEAR);
        let borrow_apr = rate_to_apr_pct(borrow_rate, SECONDS_PER_YEAR);

        // exchange_rate is scaled: 1e18 * 10^(underlying_decimals - mtoken_decimals)
        let exp_diff = m.underlying_decimals as i32 - m.mtoken_decimals as i32;
        let er_human = if exchange_rate > 0 {
            let scale = 10f64.powi(exp_diff);
            (exchange_rate as f64) / 1e18 / scale
        } else {
            0.0
        };

        markets.push(json!({
            "symbol": m.symbol,
            "mtoken": m.mtoken,
            "underlying": m.underlying,
            "supply_apr_pct": format!("{:.4}", supply_apr),
            "borrow_apr_pct": format!("{:.4}", borrow_apr),
            "exchange_rate": format!("{:.8}", er_human),
            "note": format!("1 m{} = {:.6} {}", m.symbol, er_human, m.symbol)
        }));
    }

    Ok(json!({
        "ok": true,
        "chain_id": chain_id,
        "protocol": "Moonwell Flagship",
        "markets": markets
    }))
}
