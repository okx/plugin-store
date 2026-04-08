// src/commands/markets.rs — List Compound V2 cToken markets with APR and exchange rates
use anyhow::Result;
use serde_json::{json, Value};

use crate::config::{MARKETS, BLOCKS_PER_YEAR, RPC_URL};
use crate::rpc::{supply_rate_per_block, borrow_rate_per_block, exchange_rate_current, rate_to_apr_pct};

pub async fn run(chain_id: u64) -> Result<Value> {
    if chain_id != 1 {
        anyhow::bail!("Compound V2 is only supported on Ethereum mainnet (chain 1). Got chain {}.", chain_id);
    }

    let rpc = RPC_URL;
    let mut markets = Vec::new();

    for m in MARKETS {
        let supply_rate = supply_rate_per_block(m.ctoken, rpc).await.unwrap_or(0);
        let borrow_rate = borrow_rate_per_block(m.ctoken, rpc).await.unwrap_or(0);
        let exchange_rate = exchange_rate_current(m.ctoken, rpc).await.unwrap_or(0);

        let supply_apr = rate_to_apr_pct(supply_rate, BLOCKS_PER_YEAR);
        let borrow_apr = rate_to_apr_pct(borrow_rate, BLOCKS_PER_YEAR);

        // exchange_rate is in 1e18 * (10^(underlying_decimals - ctoken_decimals)) scale
        // For display: exchange_rate / 1e18 gives cToken → underlying in raw units
        // Normalize to human-readable: divide by 10^(underlying_decimals - ctoken_decimals)
        let exp_diff = m.underlying_decimals as i32 - m.ctoken_decimals as i32;
        let er_human = if exchange_rate > 0 {
            let scale = 10f64.powi(exp_diff);
            (exchange_rate as f64) / 1e18 / scale
        } else {
            0.0
        };

        markets.push(json!({
            "symbol": m.symbol,
            "ctoken": m.ctoken,
            "underlying": m.underlying.unwrap_or("ETH (native)"),
            "supply_apr_pct": format!("{:.4}", supply_apr),
            "borrow_apr_pct": format!("{:.4}", borrow_apr),
            "exchange_rate": format!("{:.8}", er_human),
            "note": format!("1 c{} = {:.6} {}", m.symbol, er_human, m.symbol)
        }));
    }

    Ok(json!({
        "ok": true,
        "chain_id": chain_id,
        "protocol": "Compound V2",
        "markets": markets
    }))
}
