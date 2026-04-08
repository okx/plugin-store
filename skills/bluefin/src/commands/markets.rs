use clap::Args;
use serde_json::Value;

use crate::api;

#[derive(Args)]
pub struct MarketsArgs {
    /// Output raw JSON
    #[arg(long)]
    pub json: bool,

    /// Filter by specific symbol (e.g. BTC-PERP)
    #[arg(long)]
    pub symbol: Option<String>,
}

pub async fn run(args: MarketsArgs) -> anyhow::Result<()> {
    // Try to get ticker data for all markets (public, no auth needed)
    let tickers = if let Some(ref sym) = args.symbol {
        let t = api::get_ticker(sym)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch ticker for {}: {}", sym, e))?;
        // Normalize to array
        if t.is_array() {
            t
        } else {
            Value::Array(vec![t])
        }
    } else {
        // Try ticker endpoint; fall back to market data
        match api::get_all_tickers().await {
            Ok(t) => t,
            Err(_) => api::get_market_data()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch market data: {}", e))?,
        }
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&tickers)?);
        return Ok(());
    }

    let arr = tickers.as_array().cloned().unwrap_or_else(|| vec![tickers]);

    if arr.is_empty() {
        println!("No markets data returned. The Bluefin API may be temporarily unavailable.");
        println!("Check status at: https://status.bluefin.io");
        return Ok(());
    }

    println!(
        "{:<16} {:>14} {:>12} {:>12} {:>10} {:>12}",
        "Symbol", "Last Price", "24h High", "24h Low", "24h Change", "24h Volume"
    );
    println!("{}", "-".repeat(80));

    for item in &arr {
        let symbol = item["symbol"].as_str().unwrap_or("?");
        let last = format_price(item.get("lastPrice").or(item.get("close")));
        let high = format_price(item.get("high"));
        let low = format_price(item.get("low"));
        let pct = format_pct(item.get("priceChangePercent"));
        let vol = format_volume(item.get("baseAssetVolume").or(item.get("volume")));

        println!(
            "{:<16} {:>14} {:>12} {:>12} {:>10} {:>12}",
            symbol, last, high, low, pct, vol
        );
    }

    println!("\nTotal markets: {}", arr.len());
    Ok(())
}

fn format_price(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => {
            if let Ok(f) = s.parse::<f64>() {
                format!("${:.4}", f)
            } else {
                s.clone()
            }
        }
        Some(Value::Number(n)) => format!("${:.4}", n.as_f64().unwrap_or(0.0)),
        _ => "N/A".to_string(),
    }
}

fn format_pct(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => {
            if let Ok(f) = s.parse::<f64>() {
                format!("{:+.2}%", f)
            } else {
                s.clone()
            }
        }
        Some(Value::Number(n)) => format!("{:+.2}%", n.as_f64().unwrap_or(0.0)),
        _ => "N/A".to_string(),
    }
}

fn format_volume(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => {
            if let Ok(f) = s.parse::<f64>() {
                if f >= 1_000_000.0 {
                    format!("{:.2}M", f / 1_000_000.0)
                } else if f >= 1_000.0 {
                    format!("{:.2}K", f / 1_000.0)
                } else {
                    format!("{:.2}", f)
                }
            } else {
                s.clone()
            }
        }
        Some(Value::Number(n)) => {
            let f = n.as_f64().unwrap_or(0.0);
            if f >= 1_000_000.0 {
                format!("{:.2}M", f / 1_000_000.0)
            } else if f >= 1_000.0 {
                format!("{:.2}K", f / 1_000.0)
            } else {
                format!("{:.2}", f)
            }
        }
        _ => "N/A".to_string(),
    }
}
