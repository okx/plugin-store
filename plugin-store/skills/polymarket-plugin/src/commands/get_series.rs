use anyhow::Result;
use reqwest::Client;

use crate::sanitize::sanitize_opt_owned;
use crate::series::{self, seconds_remaining_in_session, seconds_until_trading_opens, SERIES};

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
                "note": "5m and 15m series: NYSE hours only. 4h series: 24/7.",
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

    let (in_hours, current, next) = series::get_series_info(&client, spec).await?;

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

            // Flat Up/Down fields for direct agent use (e.g. buy --token-id <up_token_id>)
            let up_idx = outcomes.iter().position(|o| o.to_lowercase() == "up");
            let down_idx = outcomes.iter().position(|o| o.to_lowercase() == "down");
            let up_token_id = up_idx.and_then(|i| token_ids.get(i)).cloned();
            let down_token_id = down_idx.and_then(|i| token_ids.get(i)).cloned();
            let up_price = up_idx.and_then(|i| prices.get(i)).and_then(|p| p.parse::<f64>().ok());
            let down_price = down_idx.and_then(|i| prices.get(i)).and_then(|p| p.parse::<f64>().ok());

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

    let (session_note, trading_hours_str, interval_str) = if !spec.nyse_hours_only {
        (
            "24/7 — market open".to_string(),
            "24/7",
            format!("{} hours", spec.interval_secs / 3600),
        )
    } else if in_hours {
        let secs = seconds_remaining_in_session(now);
        (
            format!("in trading hours — {}m {}s remaining in session", secs / 60, secs % 60),
            "9:30 AM – 4:00 PM ET, Monday–Friday",
            format!("{} minutes", spec.interval_secs / 60),
        )
    } else {
        let secs = seconds_until_trading_opens(now);
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        (
            format!("outside trading hours — next session opens in ~{}h {}m", h, m),
            "9:30 AM – 4:00 PM ET, Monday–Friday",
            format!("{} minutes", spec.interval_secs / 60),
        )
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
