use anyhow::Result;
use reqwest::Client;

use crate::api::{check_clob_access, list_gamma_markets, GammaMarket};
use crate::sanitize::sanitize_opt_owned;

pub async fn run(limit: u32, keyword: Option<&str>) -> Result<()> {
    let client = Client::new();

    // Geo-access check: probe the CLOB before showing markets.
    // Fail-open — a network error does not block browsing.
    let access_warning = check_clob_access(&client).await;

    let markets = list_gamma_markets(&client, limit, 0, keyword).await?;

    let output: Vec<serde_json::Value> = markets
        .iter()
        .map(|m| format_market(m))
        .collect();

    let mut data = serde_json::json!({
        "count": output.len(),
        "markets": output
    });

    if let Some(warning) = access_warning {
        data["access_warning"] = serde_json::Value::String(warning);
    }

    let result = serde_json::json!({
        "ok": true,
        "data": data
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
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
