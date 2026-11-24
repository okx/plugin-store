use anyhow::Result;
use reqwest::Client;

use crate::sanitize::sanitize_opt_owned;
use crate::api;
use crate::series::{self, SERIES};

pub async fn run(series_id: Option<&str>, list: bool) -> Result<()> {
    match run_inner(series_id, list).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("get-series"), None)); Ok(()) }
    }
}

async fn run_inner(series_id: Option<&str>, list: bool) -> Result<()> {
    // --list: print all supported series and exit
    if list || series_id.is_none() {
        let supported: Vec<serde_json::Value> = SERIES.iter().map(|s| {
            let interval_human = if s.interval_secs >= 3600 {
                format!("{} hours", s.interval_secs / 3600)
            } else {
                format!("{} minutes", s.interval_secs / 60)
            };
            serde_json::json!({
                "id": s.id,
                "asset": s.display,
                "interval": interval_human,
                "trading_hours": if s.nyse_hours_only { "NYSE hours (9:30 AM – 4:00 PM ET, Mon–Fri)" } else { "24/7" },
                "slug_pattern": format!("{}-updown-{}-{{unix_start_utc}}", s.asset, s.interval_label),
                "usage": format!("polymarket buy --market-id {} --outcome up --amount 50", s.id),
            })
        }).collect();

        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "note": "All crypto Up/Down series quote 24/7. Query a single series to see whether a slot is currently accepting orders.",
                "supported_series": supported,
            }
        }))?);
        return Ok(());
    }

    let id = series_id.unwrap();
    let spec = series::parse_series(id)
        .ok_or_else(|| anyhow::anyhow!(
            "Unknown series '{}'. Run `polymarket get-series --list` to see supported series.",
            id
        ))?;

    let client = Client::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (_in_hours, current, next) = series::get_series_info(&client, spec).await?;

    // Live quotes come from the CLOB book, not Gamma's cached outcomePrices.
    let (current_quotes, next_quotes) = tokio::join!(
        slot_quotes(&client, &current),
        slot_quotes(&client, &next),
    );

    // Format a slot for JSON output
    let format_slot = |slot: &series::SlotSummary,
                       label: &str,
                       quotes: (api::BookQuote, api::BookQuote)|
     -> serde_json::Value {
        let start_iso = chrono::DateTime::from_timestamp(slot.start_unix as i64, 0)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();
        let end_iso = chrono::DateTime::from_timestamp(slot.end_unix as i64, 0)
            .map(|d| d.to_rfc3339())
            .unwrap_or_default();
        let secs_remaining = slot.end_unix.saturating_sub(now);

        if let Some(m) = &slot.market {
            let token_ids = m.token_ids();
            let prices = m.prices();
            let outcomes = m.outcome_list();

            // Build outcome map: outcome_name -> {token_id, price}
            let outcome_map: serde_json::Value = outcomes.iter().enumerate().map(|(i, name)| {
                (name.clone(), serde_json::json!({
                    "token_id": token_ids.get(i).cloned().unwrap_or_default(),
                    "price": prices.get(i).and_then(|p| p.parse::<f64>().ok()),
                }))
            }).collect::<serde_json::Map<String, serde_json::Value>>().into();

            // Flat Up/Down fields for direct agent use (e.g. buy --token-id <up_token_id>)
            let up_idx = outcomes.iter().position(|o| o.to_lowercase() == "up");
            let down_idx = outcomes.iter().position(|o| o.to_lowercase() == "down");
            let up_token_id = up_idx.and_then(|i| token_ids.get(i)).cloned();
            let down_token_id = down_idx.and_then(|i| token_ids.get(i)).cloned();
            let snapshot_up = up_idx.and_then(|i| prices.get(i)).and_then(|p| p.parse::<f64>().ok());
            let snapshot_down = down_idx.and_then(|i| prices.get(i)).and_then(|p| p.parse::<f64>().ok());

            // Prefer the live book. Gamma's outcomePrices is a cached snapshot that
            // has been observed minutes behind the book on a live 5-minute market,
            // which makes it useless as a quote for a market resolving that fast.
            let (up_quote, down_quote) = quotes;
            let (up_price, down_price, price_source) =
                match (up_quote.mid(), down_quote.mid()) {
                    (Some(u), Some(d)) => (Some(u), Some(d), "clob_book"),
                    _ => (snapshot_up, snapshot_down, "gamma_snapshot"),
                };

            serde_json::json!({
                "slot": label,
                "slug": sanitize_opt_owned(&m.slug),
                "condition_id": m.condition_id,
                "question": sanitize_opt_owned(&m.question),
                "start": start_iso,
                "end": end_iso,
                "end_unix": slot.end_unix,
                "seconds_remaining": secs_remaining,
                "accepting_orders": m.accepting_orders,
                "up_token_id": up_token_id,
                "down_token_id": down_token_id,
                "up_price": up_price,
                "down_price": down_price,
                "up_bid": up_quote.best_bid,
                "up_ask": up_quote.best_ask,
                "down_bid": down_quote.best_bid,
                "down_ask": down_quote.best_ask,
                "price_source": price_source,
                "gamma_snapshot": {
                    "up_price": snapshot_up,
                    "down_price": snapshot_down,
                },
                "outcomes": outcome_map,
                "liquidity": m.liquidity,
                "volume_24hr": m.volume24hr,
                "last_trade_price": m.last_trade_price,
            })
        } else {
            serde_json::json!({
                "slot": label,
                "slug": slot.slug,
                "start": start_iso,
                "end": end_iso,
                "end_unix": slot.end_unix,
                "seconds_remaining": secs_remaining,
                "accepting_orders": false,
                "note": "market not yet created or not found",
            })
        }
    };

    let current_json = format_slot(&current, "current", current_quotes);
    let next_json = format_slot(&next, "next", next_quotes);

    // Build buy hint using the accepting slot
    let accepting_slug = if current.market.as_ref().map_or(false, |m| m.accepting_orders) {
        current.market.as_ref().and_then(|m| m.slug.as_deref().map(String::from))
    } else {
        next.market.as_ref().and_then(|m| m.slug.as_deref().map(String::from))
    };

    let buy_hint = accepting_slug.map(|slug| {
        format!(
            "polymarket buy --market-id {} --outcome up --amount <USDC>",
            slug
        )
    }).unwrap_or_else(|| format!(
        "polymarket buy --market-id {} --outcome up --amount <USDC>",
        spec.id
    ));

    // Report the session from what the market itself says, not from a hardcoded
    // calendar. A NYSE-hours model mislabelled these markets as closed while they
    // were quoting (04:41 ET, live two-sided book) and contradicted the very
    // `accepting_orders` flag in the same response.
    let interval_str = if spec.interval_secs >= 3600 {
        format!("{} hours", spec.interval_secs / 3600)
    } else {
        format!("{} minutes", spec.interval_secs / 60)
    };
    let trading_hours_str = "24/7";
    let any_accepting = current.market.as_ref().map_or(false, |m| m.accepting_orders)
        || next.market.as_ref().map_or(false, |m| m.accepting_orders);
    let session_note = if any_accepting {
        "open — a slot is accepting orders".to_string()
    } else {
        "no slot is currently accepting orders".to_string()
    };

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "series": spec.id,
            "asset": spec.display,
            "interval": interval_str,
            "trading_hours": trading_hours_str,
            "session": session_note,
            "current_slot": current_json,
            "next_slot": next_json,
            "tip": buy_hint,
        }
    }))?);

    Ok(())
}

/// Live CLOB book quotes for a slot's Up and Down tokens.
///
/// Pairs token ids with the outcome that names them (Gamma does not guarantee
/// the order of `outcomes`). Returns empty quotes when the slot has no market,
/// leaving the caller on the Gamma snapshot.
async fn slot_quotes(
    client: &Client,
    slot: &series::SlotSummary,
) -> (api::BookQuote, api::BookQuote) {
    let market = match &slot.market {
        Some(m) => m,
        None => return (api::BookQuote::default(), api::BookQuote::default()),
    };
    let token_ids = market.token_ids();
    let outcomes = market.outcome_list();
    let idx_of = |name: &str| outcomes.iter().position(|o| o.eq_ignore_ascii_case(name));
    let up_token = token_ids.get(idx_of("up").unwrap_or(0)).cloned().unwrap_or_default();
    let down_token = token_ids.get(idx_of("down").unwrap_or(1)).cloned().unwrap_or_default();
    tokio::join!(
        api::get_book_quote(client, &up_token),
        api::get_book_quote(client, &down_token),
    )
}
