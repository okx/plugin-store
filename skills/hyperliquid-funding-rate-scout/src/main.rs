mod api;
mod scout;
mod signal;
mod render;

use anyhow::Result;
use clap::Parser;

/// Scan Hyperliquid perpetual futures markets for extreme funding rate imbalances
/// and generate mean reversion signal cards.
#[derive(Parser, Debug)]
#[command(name = "hyperliquid-funding-rate-scout")]
#[command(version = "1.0.0")]
#[command(about = "Scan Hyperliquid perp markets for extreme funding rate imbalances")]
pub struct Args {
    /// Minimum hourly funding rate threshold in % (default: 0.01)
    #[arg(short, long, default_value = "0.01")]
    pub threshold: f64,

    /// Maximum number of signals to return (default: 5)
    #[arg(short, long, default_value = "5")]
    pub limit: usize,

    /// Filter to specific assets e.g. --asset BTC --asset ETH
    #[arg(short, long)]
    pub asset: Vec<String>,

    /// Output raw JSON instead of formatted signal cards
    #[arg(long)]
    pub json: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Convert threshold from percentage to decimal (0.01% -> 0.0001)
    let threshold_decimal = args.threshold / 100.0;

    let asset_filter: Vec<String> = args.asset.iter().map(|a| a.to_uppercase()).collect();

    let mut scout = scout::Scout::new(threshold_decimal, args.limit, asset_filter);
    let signals = scout.scan()?;

    if signals.is_empty() {
        render::render_no_signals(threshold_decimal, scout.last_scanned_count);
        return Ok(());
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&signals)?);
    } else {
        render::render_signal_cards(&signals, scout.last_scanned_count, threshold_decimal);
    }

    Ok(())
}