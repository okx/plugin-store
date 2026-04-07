use crate::api;
use crate::config::get_chain_config;
use crate::onchainos;
use anyhow::Result;

pub async fn run(chain_id: u64, account: Option<String>) -> Result<()> {
    let cfg = get_chain_config(chain_id)?;

    let wallet = match account {
        Some(a) => a,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    let data = api::get_positions(cfg.api_base_url, &wallet).await?;

    // The GMX API returns all positions; filter by account address
    let all_positions = if let Some(arr) = data.as_array() {
        arr.clone()
    } else if let Some(arr) = data["positions"].as_array() {
        arr.clone()
    } else {
        vec![]
    };

    let wallet_lower = wallet.to_lowercase();
    let positions: Vec<_> = all_positions
        .iter()
        .filter(|p| {
            p["account"]
                .as_str()
                .map(|a| a.to_lowercase() == wallet_lower)
                .unwrap_or(false)
        })
        .collect();

    if positions.is_empty() {
        println!("No open positions found for {}", wallet);
        return Ok(());
    }

    println!("Open positions for {}", wallet);
    println!("{}", "-".repeat(100));
    println!(
        "{:<44} {:<8} {:<20} {:<20}",
        "Market", "Side", "Size (USD)", "PnL"
    );
    println!("{}", "-".repeat(100));

    for p in &positions {
        let market = p["marketAddress"]
            .as_str()
            .unwrap_or(p["market"].as_str().unwrap_or("?"));
        let is_long = p["isLong"].as_bool().unwrap_or(false);
        let side = if is_long { "LONG" } else { "SHORT" };
        let size = p["sizeInUsd"]
            .as_str()
            .or_else(|| p["size"].as_str())
            .unwrap_or("0");
        let pnl = p["pnl"]
            .as_str()
            .or_else(|| p["unrealisedPnl"].as_str())
            .unwrap_or("0");

        println!("{:<44} {:<8} {:<20} {:<20}", market, side, size, pnl);
    }
    Ok(())
}
