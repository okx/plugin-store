use anyhow::Context;
use clap::Args;

use crate::api;

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address (Sui address starting with 0x...)
    #[arg(long, required = true)]
    pub wallet: String,

    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    // Fetch open positions
    let positions = api::get_user_positions(&args.wallet)
        .await
        .with_context(|| format!("Failed to fetch positions for {}", args.wallet))?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&positions)?);
        return Ok(());
    }

    let arr = positions.as_array().cloned().unwrap_or_default();

    if arr.is_empty() {
        println!("No open positions found for {}", args.wallet);
        // Also show account summary
        match api::get_account(&args.wallet).await {
            Ok(acct) => {
                let balance = acct["freeCollateral"]
                    .as_str()
                    .or_else(|| acct["totalCollateralValue"].as_str())
                    .unwrap_or("N/A");
                println!("Free collateral: {}", balance);
            }
            Err(_) => {}
        }
        return Ok(());
    }

    println!("Open positions for: {}", args.wallet);
    println!(
        "{:<16} {:>8} {:>14} {:>14} {:>14} {:>12} {:>10}",
        "Symbol", "Side", "Size", "Entry Price", "Mark Price", "Unrealized PnL", "Leverage"
    );
    println!("{}", "-".repeat(95));

    for pos in &arr {
        let symbol = pos["symbol"].as_str().unwrap_or("?");
        let side = if pos["side"].as_str().unwrap_or("").to_lowercase() == "buy"
            || pos["positionSide"].as_str().unwrap_or("").to_lowercase() == "long"
        {
            "LONG"
        } else {
            "SHORT"
        };
        let size = pos["positionQty"]
            .as_str()
            .or_else(|| pos["quantity"].as_str())
            .unwrap_or("?");
        let entry = pos["avgEntryPrice"]
            .as_str()
            .or_else(|| pos["entryPrice"].as_str())
            .unwrap_or("?");
        let mark = pos["markPrice"].as_str().unwrap_or("?");
        let pnl = pos["unrealizedProfit"]
            .as_str()
            .or_else(|| pos["unrealizedPnl"].as_str())
            .unwrap_or("?");
        let lev = pos["leverage"].as_str().unwrap_or("?");

        println!(
            "{:<16} {:>8} {:>14} {:>14} {:>14} {:>12} {:>10}",
            symbol, side, size, entry, mark, pnl, lev
        );
    }

    println!("\nTotal positions: {}", arr.len());

    // Try to show account summary
    if let Ok(acct) = api::get_account(&args.wallet).await {
        println!("\n--- Account Summary ---");
        if let Some(v) = acct["totalCollateralValue"].as_str() {
            println!("Total collateral: {}", v);
        }
        if let Some(v) = acct["freeCollateral"].as_str() {
            println!("Free collateral:  {}", v);
        }
        if let Some(v) = acct["totalUnrealizedProfit"].as_str() {
            println!("Unrealized PnL:   {}", v);
        }
        if let Some(v) = acct["accountLeverage"].as_str() {
            println!("Account leverage: {}x", v);
        }
    }

    Ok(())
}
