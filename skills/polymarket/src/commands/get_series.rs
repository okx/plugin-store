use anyhow::Result;
use reqwest::Client;

use crate::sanitize::sanitize_opt_owned;
use crate::series::{self, seconds_remaining_in_session, seconds_until_trading_opens, SERIES};

pub async fn run(series_id: Option<&str>, list: bool) -> Result<()> {
    // --list: print all supported series and exit
    if list || series_id.is_none() {
        let supported: Vec<serde_json::Value> = SERIES.iter().map(|s| {
            serde_json::json!({
                "id": s.id,
                "asset": s.display,
                "interval": format!("{} minutes", s.interval_secs / 60),
                "slug_pattern": format!("{}-updown-{}-{{unix_start_utc}}", s.asset, s.interval_label),
                "usage": format!("polymarket buy --market-id {} --outcome up --amount 50", s.id),
            })
        }).collect();

        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "trading_hours": "9:30 AM – 4:00 PM ET, Monday–Friday (NYSE hours)",
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

    let in_trading_hours = series::is_in_trading_hours(now);
    let (_, current, next) = series::get_series_info(&client, spec).await?;

    // Format a slot for JSON output
    let format_slot = |slot: &series::SlotSummary, label: &str| -> serde_json::Value {
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

            serde_json::json!({
                "slot": label,
                "slug": sanitize_opt_owned(&m.slug),
                "condition_id": m.condition_id,
                "question": sanitize_opt_owned(&m.question),
                "start": start_iso,
                "end": end_iso,
                "seconds_remaining": secs_remaining,
                "accepting_orders": m.accepting_orders,
                "outcomes": outcome_map,
                "liquidity": m.liquidity,
                "volume_24hr": m.volume24hr,
                "best_bid": m.best_bid,
                "best_ask": m.best_ask,
                "last_trade_price": m.last_trade_price,
            })
        } else {
            serde_json::json!({
                "slot": label,
                "slug": slot.slug,
                "start": start_iso,
                "end": end_iso,
                "seconds_remaining": secs_remaining,
                "accepting_orders": false,
                "note": "market not yet created or not found",
            })
        }
    };

    let current_json = format_slot(&current, "current");
    let next_json = format_slot(&next, "next");

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

    let session_note = if in_trading_hours {
        let secs = seconds_remaining_in_session(now);
        format!("in trading hours — {}m {}s remaining in session", secs / 60, secs % 60)
    } else {
        let secs = seconds_until_trading_opens(now);
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("outside trading hours — next session opens in ~{}h {}m", h, m)
    };

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "data": {
            "series": spec.id,
            "asset": spec.display,
            "interval": format!("{} minutes", spec.interval_secs / 60),
            "trading_hours": "9:30 AM – 4:00 PM ET, Monday–Friday",
            "session": session_note,
            "current_slot": current_json,
            "next_slot": next_json,
            "tip": buy_hint,
        }
    }))?);

    Ok(())
}
