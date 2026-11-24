use clap::Args;
use serde_json::{json, Value};

use crate::config::{ARB_KNOWN_MARKETS, SUPPORTED_CHAINS, token_decimals};
use crate::onchainos::resolve_wallet;
use crate::rpc::{
    fmt_token_amount, get_account_values, get_account_wei,
    get_earnings_rate, get_market_borrow_rate, rate_to_apy, supply_rate_from,
};

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (default: onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,
    /// Account number to inspect (0 = main; isolated borrow positions use other numbers)
    #[arg(long, default_value = "0")]
    pub account_number: u128,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let wallet = match args.address {
        Some(a) => a,
        None => match resolve_wallet(chain.id) {
            Ok(a) => a,
            Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
                "Run `onchainos wallet addresses` to verify login or pass --address."),
        },
    };

    // Get aggregate USD-equiv values first (single RPC, fastest).
    // EVM-012: distinguish RPC failure from "no positions". Silent (0, 0)
    // fallback rendered as "you have no Dolomite positions" — misleading
    // users with active positions whenever the public RPC blipped.
    let (supply_value, borrow_value) = match get_account_values(
        chain.dolomite_margin, &wallet, args.account_number, chain.rpc,
    ).await {
        Ok(t) => t,
        Err(e) => return print_err(
            &format!("Failed to read aggregate account values from DolomiteMargin on {}: {:#}", chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };

    // EVM-012: keep the soft 85% default (display-only APY) but expose the
    // RPC failure so callers can mark the supply APY as best-effort.
    let (earnings_rate, earnings_rate_query_error) =
        match get_earnings_rate(chain.dolomite_margin, chain.rpc).await {
            Ok(v) => (v, None),
            Err(e) => (850_000_000_000_000_000u128, Some(format!("{:#}", e))),
        };

    // Per-market scan (parallel) — only show non-zero positions
    let futs: Vec<_> = ARB_KNOWN_MARKETS.iter().map(|(mid, sym, _)| {
        let chain = chain.clone();
        let wallet = wallet.clone();
        let mid = *mid as u128; let sym = *sym;
        async move {
            let pos_fut = get_account_wei(chain.dolomite_margin, &wallet, args.account_number, mid, chain.rpc);
            let rate_fut = get_market_borrow_rate(chain.dolomite_margin, mid, chain.rpc);
            let (p, r) = tokio::join!(pos_fut, rate_fut);
            (mid, sym, p.ok(), r.ok())
        }
    }).collect();
    let results = futures::future::join_all(futs).await;

    let mut entries: Vec<Value> = Vec::new();
    let mut partial_markets: Vec<Value> = Vec::new();
    for (mid, sym, pos, borrow_rate) in results {
        // EVM-012: track per-market RPC failures so they don't silently
        // disappear via the L58 zero-filter. Sign defaults to true (supply)
        // for the rendering branch, but the entry itself is moved to
        // `partial_markets` so the user is aware data is missing.
        let (sign, value) = match pos {
            Some(t) => t,
            None => {
                partial_markets.push(json!({
                    "market_id": mid,
                    "symbol": sym,
                    "error": "get_account_wei RPC failed",
                }));
                continue;
            }
        };
        if value == 0 { continue; }
        let dec = token_decimals(sym).unwrap_or(18);
        let kind = if sign { "supply" } else { "borrow" };
        let apy_pct = if sign {
            // supply position — show derived supply APY
            borrow_rate.map(|br| supply_rate_from(br, earnings_rate)).map(|r| format!("{:.4}", rate_to_apy(r) * 100.0))
        } else {
            // borrow position — show borrow APY (cost)
            borrow_rate.map(|r| format!("{:.4}", rate_to_apy(r) * 100.0))
        };
        entries.push(json!({
            "market_id": mid,
            "symbol": sym,
            "kind": kind,
            "amount":     fmt_token_amount(value, dec),
            "amount_raw": value.to_string(),
            "apy_pct": apy_pct,
        }));
    }

    // Health factor approximation: borrowValue / supplyValue (lower = safer; >1 = under-collateralized).
    // Dolomite uses Monetary.Value scaled to 1e36. Display ratio + USD-equiv sums.
    let supply_usd_approx = supply_value as f64 / 1e36;
    let borrow_usd_approx = borrow_value as f64 / 1e36;
    let utilization = if supply_value > 0 {
        Some((borrow_value as f64 / supply_value as f64))
    } else { None };

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "chain": chain.key,
        "chain_id": chain.id,
        "wallet": wallet,
        "account_number": args.account_number,
        "supply_usd_approx": format!("{:.4}", supply_usd_approx),
        "borrow_usd_approx": format!("{:.4}", borrow_usd_approx),
        "supply_value_raw": supply_value.to_string(),
        "borrow_value_raw": borrow_value.to_string(),
        "utilization": utilization.map(|u| format!("{:.4}", u)),
        "position_count": entries.len(),
        "positions": entries,
        "partial_markets": partial_markets,
        "earnings_rate_query_error": earnings_rate_query_error,
        "note": "Account number 0 is the main account. Isolated borrow positions use other account numbers; pass --account-number N to inspect them.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
