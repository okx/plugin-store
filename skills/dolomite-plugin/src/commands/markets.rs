use clap::Args;
use serde_json::{json, Value};

use crate::config::{ARB_KNOWN_MARKETS, SUPPORTED_CHAINS, token_decimals};
use crate::rpc::{
    fmt_token_amount, get_earnings_rate, get_market_borrow_rate,
    get_market_total_par, get_num_markets, rate_to_apy, supply_rate_from,
};

#[derive(Args)]
pub struct MarketsArgs {
    /// Show all on-chain markets, not just the well-known whitelist (slower — ~1 RPC per market)
    #[arg(long)]
    pub all: bool,

    /// Limit number of markets to fetch when --all is set (default 30; Dolomite has 80+)
    #[arg(long, default_value = "30")]
    pub limit: u128,
}

pub async fn run(args: MarketsArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    // Default mode: only the well-known whitelist (fast, no per-market RPC for symbols)
    let market_ids: Vec<(u128, &'static str)> = if args.all {
        let n = match get_num_markets(chain.dolomite_margin, chain.rpc).await {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("Failed to fetch market count: {:#}", e),
                    "RPC_ERROR",
                    "Public Arbitrum RPC may be limited; retry shortly.",
                ));
                return Ok(());
            }
        };
        let cap = n.min(args.limit);
        (0..cap).map(|i| (i, "?")).collect()  // unknown symbol — caller can `quickstart` to see decoded
    } else {
        ARB_KNOWN_MARKETS.iter().map(|(mid, sym, _)| (*mid as u128, *sym)).collect()
    };

    // Earnings rate is global — read once. EVM-012: keep the soft 85% default
    // (display-only field; APY rendering is non-critical) but surface the
    // RPC failure so callers can mark the rendered APY as best-effort.
    let (earnings_rate, earnings_rate_query_error) =
        match get_earnings_rate(chain.dolomite_margin, chain.rpc).await {
            Ok(v) => (v, None),
            Err(e) => (850_000_000_000_000_000u128, Some(format!("{:#}", e))),
        };

    // Parallel: per-market borrow rate + total par
    let futs: Vec<_> = market_ids.iter().map(|(mid, sym)| {
        let chain = chain.clone();
        let mid = *mid; let sym = *sym;
        async move {
            let borrow_fut = get_market_borrow_rate(chain.dolomite_margin, mid, chain.rpc);
            let total_fut = get_market_total_par(chain.dolomite_margin, mid, chain.rpc);
            let (b, t) = tokio::join!(borrow_fut, total_fut);
            (mid, sym, b.ok(), t.ok())
        }
    }).collect();
    let results = futures::future::join_all(futs).await;

    let entries: Vec<Value> = results.into_iter().map(|(mid, sym, b_rate, total)| {
        let dec = token_decimals(sym).unwrap_or(18);
        // EVM-012: track per-market RPC failures so callers can tell "0
        // supply / 0 borrow" from "RPC failed". Silent (0,0) used to break
        // utilization analysis and hide active markets behind zero values.
        let mut errs: Vec<String> = Vec::new();
        let (sp, bp) = match total {
            Some(t) => t,
            None => { errs.push("get_market_total_par".into()); (0, 0) }
        };
        if b_rate.is_none() { errs.push("get_market_borrow_rate".into()); }
        let supply_apy = b_rate.map(|br| supply_rate_from(br, earnings_rate)).map(rate_to_apy);
        let borrow_apy = b_rate.map(rate_to_apy);
        json!({
            "market_id": mid,
            "symbol": sym,
            "supply_apy_pct":  supply_apy.map(|a| format!("{:.4}", a * 100.0)),
            "borrow_apy_pct":  borrow_apy.map(|a| format!("{:.4}", a * 100.0)),
            "total_supply":    fmt_token_amount(sp, dec),
            "total_supply_raw": sp.to_string(),
            "total_borrow":    fmt_token_amount(bp, dec),
            "total_borrow_raw": bp.to_string(),
            "utilization_pct": if sp > 0 {
                Some(format!("{:.2}", (bp as f64 / sp as f64) * 100.0))
            } else { None },
            "partial_data_errors": if errs.is_empty() { None } else { Some(errs) },
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "chain": chain.key,
        "chain_id": chain.id,
        "source": if args.all { "live_enumeration" } else { "well_known_whitelist" },
        "count": entries.len(),
        "markets": entries,
        "earnings_rate_query_error": earnings_rate_query_error,
        "note": if !args.all { "Showing 7 most-common markets. Use --all for full on-chain enumeration (~30+ markets)." } else { "" },
    }))?);
    Ok(())
}
