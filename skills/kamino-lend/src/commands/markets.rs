use clap::Args;
use serde_json::Value;

use crate::{api, config};

#[derive(Args)]
pub struct MarketsArgs {
    /// Filter by market name (optional, e.g. "main", "jlp")
    #[arg(long)]
    pub name: Option<String>,
}

pub async fn run(args: MarketsArgs) -> anyhow::Result<()> {
    let markets_raw = api::get_markets().await?;

    let markets = match markets_raw.as_array() {
        Some(arr) => arr.clone(),
        None => {
            anyhow::bail!("Unexpected markets response format: {}", markets_raw);
        }
    };

    let mut result_markets = Vec::new();

    for market in &markets {
        let market_pubkey = market["lendingMarket"].as_str().unwrap_or("");
        let name = market["name"].as_str().unwrap_or("");
        let is_primary = market["isPrimary"].as_bool().unwrap_or(false);

        // Filter by name if provided
        if let Some(ref filter) = args.name {
            if !name.to_lowercase().contains(&filter.to_lowercase()) {
                continue;
            }
        }

        // For the main market, fetch APY data for key reserves
        let mut reserves_info = Vec::new();
        if is_primary || market_pubkey == config::MAIN_MARKET {
            let known_reserves = [
                ("USDC", "D6q6wuQSrifJKZYpR1M8R4YawnLDtDsMmWM1NbBmgJ59"),
                ("SOL", "d4A2prbA2whesmvHaL88BH6Ewn5N4bTSU2Ze8P6Bc4Q"),
            ];
            for (symbol, reserve_addr) in &known_reserves {
                if let Ok(metrics) = api::get_reserve_metrics(market_pubkey, reserve_addr).await {
                    if let Some(latest) = get_latest_metrics(&metrics) {
                        reserves_info.push(serde_json::json!({
                            "symbol": symbol,
                            "reserve": reserve_addr,
                            "supply_apy": format_pct(latest["supplyInterestAPY"].as_f64()),
                            "borrow_apy": format_pct(latest["borrowInterestAPY"].as_f64()),
                            "deposit_tvl": format_usd(latest["depositTvl"].as_str()),
                            "borrow_tvl": format_usd(latest["borrowTvl"].as_str()),
                            "total_liquidity": latest["totalLiquidity"].as_str().unwrap_or("0"),
                            "ltv": latest["loanToValue"].as_f64().unwrap_or(0.0),
                        }));
                    }
                }
            }
        }

        result_markets.push(serde_json::json!({
            "market": market_pubkey,
            "name": name,
            "is_primary": is_primary,
            "description": market["description"].as_str().unwrap_or(""),
            "reserves": reserves_info,
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "total": result_markets.len(),
                "markets": result_markets
            }
        }))?
    );

    Ok(())
}

fn get_latest_metrics(data: &Value) -> Option<&Value> {
    data["history"].as_array()?.last().map(|entry| &entry["metrics"])
}

fn format_pct(val: Option<f64>) -> String {
    match val {
        Some(v) => format!("{:.4}%", v * 100.0),
        None => "N/A".to_string(),
    }
}

fn format_usd(val: Option<&str>) -> String {
    match val {
        Some(v) => {
            if let Ok(f) = v.parse::<f64>() {
                format!("${:.2}", f)
            } else {
                v.to_string()
            }
        }
        None => "N/A".to_string(),
    }
}
