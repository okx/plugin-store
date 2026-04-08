use anyhow::Context;
use clap::Args;

use crate::api;

#[derive(Args)]
pub struct QuoteArgs {
    /// Market symbol (e.g. BTC-PERP, ETH-PERP)
    #[arg(long, required = true)]
    pub market: String,

    /// Size to quote (in base asset units, e.g. 0.01 for 0.01 BTC)
    #[arg(long, default_value = "1")]
    pub amount: String,

    /// Side to quote: bid (buy) or ask (sell)
    #[arg(long, default_value = "ask")]
    pub side: String,

    /// Orderbook depth to display
    #[arg(long, default_value = "5")]
    pub depth: u32,

    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: QuoteArgs) -> anyhow::Result<()> {
    let symbol = args.market.to_uppercase();

    // Fetch orderbook
    let ob = api::get_orderbook(&symbol, Some(args.depth))
        .await
        .with_context(|| format!("Failed to fetch orderbook for {}", symbol))?;

    // Fetch current ticker for context
    let ticker = api::get_ticker(&symbol).await.ok();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&ob)?);
        return Ok(());
    }

    println!("=== Orderbook: {} ===", symbol);

    // Show ticker summary if available
    if let Some(t) = &ticker {
        let t = if t.is_array() {
            t.as_array()
                .and_then(|a| a.first())
                .cloned()
                .unwrap_or_default()
        } else {
            t.clone()
        };

        let last = t["lastPrice"]
            .as_str()
            .or_else(|| t["close"].as_str())
            .unwrap_or("?");
        let mark = t["markPrice"].as_str().unwrap_or("?");
        let index = t["indexPrice"].as_str().unwrap_or("?");
        println!("Last: {}  Mark: {}  Index: {}", last, mark, index);
        println!();
    }

    // Parse amount for quote calculation
    let amount: f64 = args.amount.parse().unwrap_or(1.0);

    // Display asks (sell side)
    let asks = ob["asks"].as_array().cloned().unwrap_or_default();
    let bids = ob["bids"].as_array().cloned().unwrap_or_default();

    println!("{:>6}  {:>16}  {:>16}  Note", "Level", "Ask Price", "Ask Size");
    println!("{}", "-".repeat(60));
    let show_asks: Vec<_> = asks.iter().take(args.depth as usize).collect();
    // Show asks in reverse (highest to lowest)
    for (i, level) in show_asks.iter().rev().enumerate() {
        let price = extract_price_str(level);
        let size = extract_size_str(level);
        let note = if i == show_asks.len() - 1 {
            " <- best ask"
        } else {
            ""
        };
        println!("{:>6}  {:>16}  {:>16}{}", i + 1, price, size, note);
    }

    println!("{:>6}  {:>16}  {:>16}  Note", "Level", "Bid Price", "Bid Size");
    println!("{}", "-".repeat(60));
    for (i, level) in bids.iter().take(args.depth as usize).enumerate() {
        let price = extract_price_str(level);
        let size = extract_size_str(level);
        let note = if i == 0 { " <- best bid" } else { "" };
        println!("{:>6}  {:>16}  {:>16}{}", i + 1, price, size, note);
    }

    // Compute estimated fill cost
    let side_lower = args.side.to_lowercase();
    let levels = if side_lower == "bid" || side_lower == "buy" {
        &asks
    } else {
        &bids
    };

    println!();
    println!("--- Quote for {} {} {} ---", amount, symbol, side_lower);

    let mut remaining = amount;
    let mut total_cost: f64 = 0.0;
    let mut filled: f64 = 0.0;

    for level in levels {
        if remaining <= 0.0 {
            break;
        }
        let price: f64 = extract_price_f64(level);
        let size: f64 = extract_size_f64(level);
        let take = remaining.min(size);
        total_cost += take * price;
        filled += take;
        remaining -= take;
    }

    if filled > 0.0 {
        let avg_price = total_cost / filled;
        println!("Filled:    {:.6} {}", filled, symbol.split('-').next().unwrap_or("?"));
        println!("Avg price: ${:.4}", avg_price);
        println!("Est. cost: ${:.4}", total_cost);
        if remaining > 0.0 {
            println!("WARNING: Only {:.6} of {} fillable from current orderbook depth", filled, amount);
        }
    } else {
        println!("No liquidity available at current depth");
    }

    // Show funding rate
    if let Ok(fr) = api::get_funding_rate(&symbol).await {
        let rate = fr["fundingRate"]
            .as_str()
            .or_else(|| fr["currentFundingRate"].as_str())
            .unwrap_or("?");
        println!("Funding rate: {}", rate);
    }

    Ok(())
}

fn extract_price_str(level: &serde_json::Value) -> String {
    if let Some(arr) = level.as_array() {
        arr.first()
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "?".to_string())
    } else {
        level["price"]
            .as_str()
            .unwrap_or("?")
            .to_string()
    }
}

fn extract_size_str(level: &serde_json::Value) -> String {
    if let Some(arr) = level.as_array() {
        arr.get(1)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "?".to_string())
    } else {
        level["quantity"]
            .as_str()
            .or_else(|| level["size"].as_str())
            .unwrap_or("?")
            .to_string()
    }
}

fn extract_price_f64(level: &serde_json::Value) -> f64 {
    let s = extract_price_str(level);
    s.parse().unwrap_or(0.0)
}

fn extract_size_f64(level: &serde_json::Value) -> f64 {
    let s = extract_size_str(level);
    s.parse().unwrap_or(0.0)
}
