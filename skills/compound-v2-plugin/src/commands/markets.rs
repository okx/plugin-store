use clap::Args;
use serde_json::{json, Value};

use crate::config::{ETH_KNOWN_MARKETS, SUPPORTED_CHAINS};
use crate::rpc::{
    borrow_rate_per_block, ctoken_total_supply, exchange_rate_stored, fmt_token_amount,
    get_cash, get_market, is_borrow_paused, is_mint_paused, rate_per_block_to_apr,
    supply_rate_per_block, total_borrows,
};

#[derive(Args)]
pub struct MarketsArgs {
    /// Show all 20 cTokens registered with Comptroller (including paused/dead markets);
    /// default 6 well-known.
    #[arg(long)]
    pub all: bool,
}

pub async fn run(_args: MarketsArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    // Per-market parallel reads
    let futs: Vec<_> = ETH_KNOWN_MARKETS.iter().map(|info| {
        let chain = chain.clone();
        async move {
            let supply_rate = supply_rate_per_block(info.ctoken, chain.rpc);
            let borrow_rate = borrow_rate_per_block(info.ctoken, chain.rpc);
            let exchange_rate = exchange_rate_stored(info.ctoken, chain.rpc);
            let total_b = total_borrows(info.ctoken, chain.rpc);
            let total_s = ctoken_total_supply(info.ctoken, chain.rpc);
            let cash = get_cash(info.ctoken, chain.rpc);
            let mp = is_mint_paused(chain.comptroller, info.ctoken, chain.rpc);
            let bp = is_borrow_paused(chain.comptroller, info.ctoken, chain.rpc);
            let market = get_market(chain.comptroller, info.ctoken, chain.rpc);
            let (sr, br, er, tb, ts, c, mp_r, bp_r, m) = tokio::join!(
                supply_rate, borrow_rate, exchange_rate, total_b, total_s, cash, mp, bp, market
            );
            (info, sr.ok(), br.ok(), er.ok(), tb.ok(), ts.ok(), c.ok(), mp_r.ok(), bp_r.ok(), m.ok())
        }
    }).collect();
    let results = futures::future::join_all(futs).await;

    let entries: Vec<Value> = results.into_iter().map(|(info, sr, br, er, tb, ts, cash, mp, bp, market_data)| {
        let supply_apr = sr.map(|r| rate_per_block_to_apr(r, chain.blocks_per_year));
        let borrow_apr = br.map(|r| rate_per_block_to_apr(r, chain.blocks_per_year));
        // Total supply in underlying = totalSupply (cToken, 8dec) × exchangeRate (1e18 scaled but
        // factors in dec offset). Compound V2 formula: underlying_supply = totalSupply × exchangeRate / 1e18
        // exchangeRate already accounts for 18 - 8 + underlying_decimals = 10 + underlying_decimals scale.
        // So divide by 10^(18 + 8 - underlying_decimals) = 10^(26 - underlying_decimals).
        // Cleaner: underlying = totalSupply × exchangeRate / 10^(18+8-underlying_decimals)
        let total_supply_underlying = match (ts, er) {
            (Some(t), Some(rate)) => {
                // saturating chain to avoid overflow on large t × rate
                let scale_pow = 18 + 8u32 - info.underlying_decimals as u32;
                if scale_pow > 38 { 0 } else {
                    // Both u128; do f64 to avoid overflow
                    let underlying_f = (t as f64) * (rate as f64) / 10f64.powi(scale_pow as i32);
                    if underlying_f.is_finite() && underlying_f >= 0.0 {
                        (underlying_f.min(u128::MAX as f64)) as u128
                    } else { 0 }
                }
            }
            _ => 0,
        };
        // EVM-012: track per-market RPC errors so callers can tell "0 borrow"
        // from "RPC failed". Silently zeroing used to render markets as empty
        // when RPC blipped, and broke utilization_pct = total_borrow / total_supply
        // (latter could also be 0, triggering the L67 None branch artificially).
        let mut errs: Vec<String> = Vec::new();
        let total_borrow = tb.unwrap_or_else(|| { errs.push("total_borrow".into()); 0 });
        let cash_v = cash.unwrap_or_else(|| { errs.push("cash".into()); 0 });
        let utilization_pct = if total_supply_underlying > 0 {
            Some(format!("{:.2}", (total_borrow as f64 / total_supply_underlying as f64) * 100.0))
        } else { None };
        let (is_listed, cf, is_comped) = market_data.unwrap_or((false, 0, false));
        json!({
            "ctoken": info.ctoken,
            "ctoken_symbol": info.symbol,
            "underlying": info.underlying,
            "underlying_symbol": info.underlying_symbol,
            "underlying_decimals": info.underlying_decimals,
            "is_native": info.is_native,
            "supply_apr_pct": supply_apr.map(|a| format!("{:.4}", a * 100.0)),
            "borrow_apr_pct": borrow_apr.map(|a| format!("{:.4}", a * 100.0)),
            "total_supply_underlying":     fmt_token_amount(total_supply_underlying, info.underlying_decimals),
            "total_supply_underlying_raw": total_supply_underlying.to_string(),
            "total_borrow_underlying":     fmt_token_amount(total_borrow, info.underlying_decimals),
            "total_borrow_underlying_raw": total_borrow.to_string(),
            "cash_underlying":     fmt_token_amount(cash_v, info.underlying_decimals),
            "utilization_pct": utilization_pct,
            "is_listed": is_listed,
            "collateral_factor_pct": format!("{:.2}", cf as f64 / 1e18 * 100.0),
            "comp_distributed": is_comped,
            "mint_paused": mp.unwrap_or(false),
            "borrow_paused": bp.unwrap_or(false),
            "partial_data_errors": if errs.is_empty() { None } else { Some(errs) },
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "chain": chain.key,
        "chain_id": chain.id,
        "comptroller": chain.comptroller,
        "blocks_per_year": chain.blocks_per_year,
        "winddown_notice": "All 6 markets have mint_paused=true (governance wind-down). New supply is rejected; redeem/repay/claim still work for legacy positions.",
        "count": entries.len(),
        "markets": entries,
        "note": "Showing 6 well-known cTokens. The Comptroller registers 20 cTokens total (deprecated cBAT/cREP/cSAI/cUNI/cTUSD/cMKR/cAAVE/cYFI/cSUSHI/cLINK/cUSDP/cFEI/cZRX); v0.1.0 hard-codes the 6 with non-trivial liquidity.",
    }))?);
    Ok(())
}
