use anyhow::Result;
use reqwest::Client;

use crate::api::{list_breaking_events, list_category_events, list_gamma_markets, GammaMarket};
use crate::sanitize::sanitize_opt_owned;

pub async fn run(
    limit: u32,
    keyword: Option<&str>,
    breaking: bool,
    category: Option<&str>,
) -> Result<()> {
    match run_inner(limit, keyword, breaking, category).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("list-markets"), None)); Ok(()) }
    }
}

async fn run_inner(
    limit: u32,
    keyword: Option<&str>,
    breaking: bool,
    category: Option<&str>,
) -> Result<()> {
    let client = Client::new();

    // --breaking or --category both use the events API
    if breaking || category.is_some() {
        let mode = category.unwrap_or("breaking");
        let events = if let Some(cat) = category {
            list_category_events(&client, cat, limit).await?
        } else {
            list_breaking_events(&client, limit).await?
        };

        let output: Vec<serde_json::Value> = events
            .iter()
            .map(|e| format_event(e))
            .collect();

        let result = serde_json::json!({
            "ok": true,
            "data": {
                "count": output.len(),
                "mode": mode,
                "events": output
            }
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let markets = list_gamma_markets(&client, limit, 0, keyword).await?;
    let output: Vec<serde_json::Value> = markets.iter().map(|m| format_market(m)).collect();
    let result = serde_json::json!({
        "ok": true,
        "data": {
            "count": output.len(),
            "mode": "top",
            "markets": output
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn format_event(e: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "title": e["title"],
        "slug": e["slug"],
        "volume_24hr": e["volume24hr"],
        "start_date": e["startDate"],
        "end_date": e["endDate"],
        "market_count": e["markets"].as_array().map(|a| a.len()).unwrap_or(0),
    })
}

fn format_market(m: &GammaMarket) -> serde_json::Value {
    let token_ids = m.token_ids();
    let prices = m.prices();
    let _outcomes = m.outcome_list();

    let yes_price = prices.first().cloned().unwrap_or_default();
    let no_price = prices.get(1).cloned().unwrap_or_default();
    let yes_token_id = token_ids.first().cloned().unwrap_or_default();
    let no_token_id = token_ids.get(1).cloned().unwrap_or_default();

    serde_json::json!({
        "question": sanitize_opt_owned(&m.question),
        "condition_id": m.condition_id,
        "slug": sanitize_opt_owned(&m.slug),
        "end_date": m.end_date,
        "active": m.active,
        "closed": m.closed,
        "accepting_orders": m.accepting_orders,
        "neg_risk": m.neg_risk,
        "yes_price": yes_price,
        "no_price": no_price,
        "yes_token_id": yes_token_id,
        "no_token_id": no_token_id,
        "volume_24hr": m.volume24hr,
        "liquidity": m.liquidity,
        "best_bid": m.best_bid,
        "best_ask": m.best_ask,
        "last_trade_price": m.last_trade_price,
    })
}
