use clap::Args;
use std::collections::HashMap;

use crate::{api, config};

#[derive(Args)]
pub struct ReservesArgs {
    /// Minimum supply APY filter (0–100, e.g. 1 = at least 1% APY)
    #[arg(long)]
    pub min_apy: Option<f64>,

    /// Minimum borrow APY filter (0–100, e.g. 1 = at least 1% borrow APY)
    #[arg(long)]
    pub min_borrow_apy: Option<f64>,
}

/// Known reserves to enrich with borrow APY from the Kamino API.
/// All other reserves (mostly LSTs) show borrow APY as null.
const KNOWN_RESERVES: &[(&str, &str)] = &[
    ("USDC",    "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59"),
    ("USDT",    "H3t6qZ1JkguCNTi9uzVKqQ7dvt2cum4XiXWom6Gn5e5S"),
    ("PYUSD",   "2gc9Dm1eB6UgVYFBUN9bWks6Kes9PbWSaPaa9DqyvEiN"),
    ("USDS",    "BHUi32TrEsfN2U821G4FprKrR4hTeK4LCWtA3BFetuqA"),
    ("SOL",     "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q"),
    ("JITOSOL", "EVbyPKrHG6WBfm4dLxLMJpUDY43cCAcHSpV3KYjKsktW"),
    ("MSOL",    "FBSyPnxtHKLBZ4UeeUyAnbtFuAmTHLtso9YtsqRDRWpM"),
    ("JUPSOL",  "DGQZWCY17gGtBUgdaFs1VreJWsodkjFxndPsskwFKGpp"),
    ("BSOL",    "H9vmCVd77N1HZa36eBn3UnftYmg4vQzPfm1RxabHAMER"),
    ("ETH",     "febGYTnFX4GbSGoFHFeJXUHgNaK53fB23uDins9Jp1E"),
    ("CBBTC",   "37Jk2zkz23vkAYBT66HM2gaqJuNg2nYLsCreQAVt5MWK"),
];

pub async fn run(args: ReservesArgs) -> anyhow::Result<()> {
    // Fetch full list from DeFiLlama (supply APY + TVL for all 44 reserves)
    let mut pools = match api::fetch_kamino_reserves_defillama().await {
        Ok(p) => p,
        Err(e) => {
            println!("{}", super::error_response(&e, None));
            return Ok(());
        }
    };

    // Concurrently fetch borrow APY from Kamino API for known reserves
    let borrow_apys = fetch_borrow_apys().await;

    // Apply filters
    if let Some(min) = args.min_apy {
        pools.retain(|p| p["apy"].as_f64().unwrap_or(0.0) >= min);
    }
    if let Some(min_borrow) = args.min_borrow_apy {
        pools.retain(|p| {
            let sym = p["symbol"].as_str().unwrap_or("").to_uppercase();
            borrow_apys.get(&sym).copied().unwrap_or(0.0) >= min_borrow
        });
    }

    // Sort by TVL descending
    pools.sort_by(|a, b| {
        let a = a["tvlUsd"].as_f64().unwrap_or(0.0);
        let b = b["tvlUsd"].as_f64().unwrap_or(0.0);
        b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal)
    });

    let reserves: Vec<serde_json::Value> = pools
        .iter()
        .map(|p| {
            let symbol = p["symbol"].as_str().unwrap_or("UNKNOWN").to_string();
            let supply_apy = p["apy"].as_f64().unwrap_or(0.0);
            let borrow_apy = borrow_apys.get(&symbol.to_uppercase()).copied();
            let tvl_usd = p["tvlUsd"].as_f64().unwrap_or(0.0);

            let mut entry = serde_json::json!({
                "symbol": symbol,
                "supply_apy_pct": format!("{:.2}", supply_apy),
                "tvl_usd": format!("{:.0}", tvl_usd),
                "supply_example": format!(
                    "kamino-lend supply --token {} --amount <amount> --confirm",
                    symbol
                ),
            });

            // Add borrow APY and borrow example only if available
            if let Some(borrow) = borrow_apy {
                entry["borrow_apy_pct"] = serde_json::json!(format!("{:.2}", borrow));
                entry["borrow_example"] = serde_json::json!(format!(
                    "kamino-lend borrow --token {} --amount <amount> --dry-run",
                    symbol
                ));
            }

            entry
        })
        .collect();

    if reserves.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "count": 0,
                "reserves": [],
                "note": "No reserves matched the filter."
            }))?
        );
        return Ok(());
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "source": "DeFiLlama + Kamino API",
            "market": "main",
            "count": reserves.len(),
            "reserves": reserves
        }))?
    );

    Ok(())
}

/// Fetch borrow APY (as percentage) for all known reserves in parallel.
/// Returns a map of SYMBOL_UPPERCASE → borrow_apy_pct.
/// Silently skips any reserve that fails (network error, no data).
async fn fetch_borrow_apys() -> HashMap<String, f64> {
    let market = config::MAIN_MARKET;
    let futures: Vec<_> = KNOWN_RESERVES
        .iter()
        .map(|(sym, addr)| {
            let sym = sym.to_string();
            async move {
                let metrics = api::get_reserve_metrics(market, addr).await.ok()?;
                let latest = metrics["history"].as_array()?.last()?;
                let borrow = latest["metrics"]["borrowInterestAPY"]
                    .as_f64()?;
                Some((sym.to_uppercase(), borrow * 100.0))
            }
        })
        .collect();

    let mut map = HashMap::new();
    for fut in futures {
        if let Some((sym, apy)) = fut.await {
            map.insert(sym, apy);
        }
    }
    map
}
