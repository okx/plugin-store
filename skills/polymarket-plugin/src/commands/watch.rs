use anyhow::{bail, Result};
use reqwest::Client;

use crate::api::{get_clob_market, get_gamma_market_by_slug, get_market_live_activity};

/// Watch live trade activity for a market, polling every `interval` seconds.
///
/// Prints new trade events as they arrive. Runs until Ctrl+C.
/// `market_id`: condition_id (0x-prefixed) or slug.
/// `interval`: seconds between polls (minimum 2, default 5).
/// `limit`: max events to fetch per poll (default 10).
pub async fn run(market_id: &str, interval: u64, limit: u32) -> Result<()> {
    if interval < 2 {
        bail!("--interval must be at least 2 seconds");
    }

    let client = Client::new();

    // Resolve market_id → condition_id + question label.
    let (condition_id, label) = if market_id.starts_with("0x") || market_id.starts_with("0X") {
        let m = get_clob_market(&client, market_id).await?;
        let q = m.question.unwrap_or_else(|| market_id.to_string());
        (m.condition_id, q)
    } else {
        let m = get_gamma_market_by_slug(&client, market_id).await?;
        let cid = m.condition_id
            .ok_or_else(|| anyhow::anyhow!("Market '{}' has no condition_id", market_id))?;
        let q = m.question.unwrap_or_else(|| market_id.to_string());
        (cid, q)
    };

    eprintln!("[polymarket] Watching: {}", label);
    eprintln!("[polymarket] Market:   {}", condition_id);
    eprintln!("[polymarket] Polling every {}s — press Ctrl+C to stop.\n", interval);

    // Track the most recent event timestamp to avoid reprinting duplicates.
    let mut last_seen_ts: Option<u64> = None;

    loop {
        match get_market_live_activity(&client, &condition_id, limit).await {
            Ok(events) => {
                // Events are newest-first; filter out already-seen timestamps.
                let new_events: Vec<_> = events
                    .iter()
                    .filter(|e| {
                        let ts = e.timestamp.unwrap_or(0);
                        last_seen_ts.map_or(true, |last| ts > last)
                    })
                    .collect();

                // Update the high-water mark.
                if let Some(newest) = events.first().and_then(|e| e.timestamp) {
                    last_seen_ts = Some(newest);
                }

                // Print in chronological order (newest-first → reverse before printing).
                for event in new_events.iter().rev() {
                    let price = event.price.as_deref().unwrap_or("?");
                    let size = event.size.as_deref().unwrap_or("?");
                    let side = event.side.as_deref().unwrap_or("?");
                    let outcome = event.outcome.as_deref().unwrap_or("?");
                    let ts = event.timestamp.unwrap_or(0);
                    println!(
                        "{}",
                        serde_json::to_string(&serde_json::json!({
                            "timestamp": ts,
                            "side": side,
                            "outcome": outcome,
                            "price": price,
                            "size": size,
                            "tx_hash": event.tx_hash,
                        }))
                        .unwrap_or_default()
                    );
                }
            }
            Err(e) => {
                eprintln!("[polymarket] Warning: poll failed ({}); retrying...", e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
    }
}
