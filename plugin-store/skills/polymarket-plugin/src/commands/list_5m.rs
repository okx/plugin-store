/// `polymarket list-5m` — list upcoming 5-minute crypto Up/Down markets.
///
/// Constructs tickers from the current 5-minute window and fetches each
/// from the Gamma API. Displays condition_id, prices, and time window (ET).
///
/// ## Supported coins
/// BTC, ETH, SOL, XRP, BNB, DOGE, HYPE
///
/// ## Missing --coin
/// Returns a structured JSON response so the Agent can ask the user.

use anyhow::{bail, Result};
use reqwest::Client;

/// Map user coin input to the lowercase prefix used in Polymarket 5M tickers.
fn resolve_coin(coin: &str) -> Option<&'static str> {
    match coin.to_uppercase().as_str() {
        "BTC" | "BITCOIN" => Some("btc"),
        "ETH" | "ETHEREUM" | "ETHER" => Some("eth"),
        "SOL" | "SOLANA" => Some("sol"),
        "XRP" | "RIPPLE" => Some("xrp"),
        "BNB" | "BINANCE" => Some("bnb"),
        "DOGE" | "DOGECOIN" => Some("doge"),
        "HYPE" | "HYPERLIQUID" => Some("hype"),
        _ => None,
    }
}

/// Format an ISO-8601 UTC timestamp as a human-readable ET window string.
/// e.g. "2026-04-13T17:55:00Z" → "April 13, 1:55PM ET"
fn format_et(iso: &str) -> String {
    // Parse the UTC timestamp manually (no chrono dep needed for this format)
    // Format: YYYY-MM-DDTHH:MM:SSZ
    let parts: Vec<&str> = iso.splitn(2, 'T').collect();
    if parts.len() != 2 {
        return iso.to_string();
    }
    let date_parts: Vec<&str> = parts[0].splitn(3, '-').collect();
    let time_parts: Vec<&str> = parts[1].trim_end_matches('Z').splitn(3, ':').collect();
    if date_parts.len() != 3 || time_parts.len() < 2 {
        return iso.to_string();
    }

    let month: u32 = date_parts[1].parse().unwrap_or(0);
    let day: u32 = date_parts[2].parse().unwrap_or(0);
    let utc_hour: i32 = time_parts[0].parse().unwrap_or(0);
    let min: u32 = time_parts[1].parse().unwrap_or(0);

    // ET = UTC-4 (EDT, currently in effect Apr-Nov)
    let et_hour = ((utc_hour - 4).rem_euclid(24)) as u32;
    let et_day = if utc_hour < 4 { day.saturating_sub(1) } else { day };

    let month_name = match month {
        1 => "January", 2 => "February", 3 => "March", 4 => "April",
        5 => "May", 6 => "June", 7 => "July", 8 => "August",
        9 => "September", 10 => "October", 11 => "November", 12 => "December",
        _ => "?",
    };

    let (display_hour, ampm) = if et_hour == 0 {
        (12, "AM")
    } else if et_hour < 12 {
        (et_hour, "AM")
    } else if et_hour == 12 {
        (12, "PM")
    } else {
        (et_hour - 12, "PM")
    };

    format!("{} {}, {}:{:02}{} ET", month_name, et_day, display_hour, min, ampm)
}

pub async fn run(coin: Option<&str>, count: u32) -> Result<()> {
    match run_inner(coin, count).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("list-5m"), None)); Ok(()) }
    }
}

async fn run_inner(coin: Option<&str>, count: u32) -> Result<()> {
    // ── Missing --coin: ask the Agent to get it from the user ────────────────
    let coin_str = match coin {
        Some(c) => c,
        None => {
            println!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "missing_params": ["coin"],
                    "error": "Missing required parameter: --coin",
                    "hint": "Please ask the user: which coin do you want to view 5-minute markets for? \
                             Supported: BTC, ETH, SOL, XRP, BNB, DOGE, HYPE"
                })
            );
            return Ok(());
        }
    };

    let coin_prefix = resolve_coin(coin_str).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown coin '{}'. Supported: BTC, ETH, SOL, XRP, BNB, DOGE, HYPE",
            coin_str
        )
    })?;

    if count == 0 || count > 20 {
        bail!("--count must be between 1 and 20");
    }

    let client = Client::new();

    // Current 5-minute window: floor(now_secs / 300) * 300
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let current_window = (now_secs / 300) * 300;

    let mut markets = Vec::new();
    let mut not_found = Vec::new();

    for i in 0..count {
        let window = current_window + i as u64 * 300;
        let slug = format!("{}-updown-5m-{}", coin_prefix, window);

        match crate::api::get_5m_market(&client, &slug).await {
            Ok(Some(m)) => markets.push(m),
            Ok(None) => not_found.push(slug),
            Err(e) => {
                eprintln!("[polymarket] Warning: failed to fetch {}: {}", slug, e);
                not_found.push(slug);
            }
        }
    }

    if markets.is_empty() {
        println!(
            "{}",
            serde_json::json!({
                "ok": false,
                "error": format!(
                    "No 5-minute {} markets found for the next {} windows. \
                     The market may not be published yet.",
                    coin_str.to_uppercase(), count
                ),
                "queried_slugs": not_found,
            })
        );
        return Ok(());
    }

    let market_list: Vec<serde_json::Value> = markets
        .iter()
        .map(|m| {
            serde_json::json!({
                "slug": m.slug,
                "conditionId": m.condition_id,
                "question": m.question,
                "timeWindow": format_et(&m.end_date),
                "endDateUtc": m.end_date,
                "upPrice": m.up_price,
                "downPrice": m.down_price,
                "upTokenId": m.up_token_id,
                "downTokenId": m.down_token_id,
                "acceptingOrders": m.accepting_orders,
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "coin": coin_str.to_uppercase(),
                "count": markets.len(),
                "markets": market_list,
                "note": format!(
                    "Use conditionId with `polymarket buy --market-id <conditionId> --outcome up --amount <usdc>` to trade."
                )
            }
        })
    );

    Ok(())
}
