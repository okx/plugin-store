use anyhow::Result;
use reqwest::Client;

use crate::api::{get_clob_market, get_gamma_market_by_slug, get_market_fee, get_orderbook};
use crate::sanitize::{sanitize_opt, sanitize_opt_owned, sanitize_str};

pub async fn run(market_id: &str) -> Result<()> {
    match run_inner(market_id).await {
        Ok(()) => Ok(()),
        Err(e) => { println!("{}", super::error_response(&e, Some("get-market"), None)); Ok(()) }
    }
}

async fn run_inner(market_id: &str) -> Result<()> {
    let client = Client::new();

    // Determine if market_id is a condition_id (0x-prefixed hex) or a slug
    let output = if market_id.starts_with("0x") || market_id.starts_with("0X") {
        run_by_condition_id(&client, market_id).await?
    } else {
        run_by_slug(&client, market_id).await?
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn run_by_condition_id(client: &Client, condition_id: &str) -> anyhow::Result<serde_json::Value> {
    let market = get_clob_market(client, condition_id).await?;

    // Fetch orderbook for each outcome token (enriches price data for all outcome types)
    let mut tokens_enriched = Vec::new();
    for t in &market.tokens {
        let book = get_orderbook(client, &t.token_id).await.ok();
        tokens_enriched.push(serde_json::json!({
            "outcome": sanitize_str(&t.outcome),
            "token_id": t.token_id,
            "price": t.price,
            "winner": t.winner,
            // CLOB returns bids ascending (last = best bid) and asks descending (last = best ask)
            "best_bid": book.as_ref().and_then(|b| b.bids.last()).map(|l| l.price.clone()),
            "best_ask": book.as_ref().and_then(|b| b.asks.last()).map(|l| l.price.clone()),
            // last_trade_price from the /book endpoint is market-level (same for all tokens)
            // and is omitted here to avoid confusion. Use last_trade_price from the Gamma
            // slug endpoint (market level) if needed.
        }));
    }

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "condition_id": market.condition_id,
            "question": sanitize_opt(market.question.as_deref()),
            "active": market.active,
            "closed": market.closed,
            "accepting_orders": market.accepting_orders,
            "neg_risk": market.neg_risk,
            "end_date": market.end_date_iso,
            "fee_bps": market.maker_base_fee,
            "tokens": tokens_enriched,
        }
    }))
}

async fn run_by_slug(client: &Client, slug: &str) -> anyhow::Result<serde_json::Value> {
    let market = get_gamma_market_by_slug(client, slug).await?;
    let token_ids = market.token_ids();
    let prices = market.prices();
    let outcomes = market.outcome_list();

    // Fetch fee from CLOB (Gamma fee field is consistently null)
    let fee_bps = if let Some(cid) = market.condition_id.as_deref() {
        get_market_fee(client, cid).await.ok()
    } else {
        None
    };

    // Enrich each outcome token with live orderbook data
    let mut token_info = Vec::new();
    for (i, outcome) in outcomes.iter().enumerate() {
        let token_id = token_ids.get(i).cloned().unwrap_or_default();
        let book = if !token_id.is_empty() {
            get_orderbook(client, &token_id).await.ok()
        } else {
            None
        };
        token_info.push(serde_json::json!({
            "outcome": sanitize_str(outcome),
            "token_id": token_id,
            "price": prices.get(i).cloned().unwrap_or_default(),
            // CLOB returns bids ascending (last = best bid) and asks descending (last = best ask)
            "best_bid": book.as_ref().and_then(|b| b.bids.last()).map(|l| l.price.clone()),
            "best_ask": book.as_ref().and_then(|b| b.asks.last()).map(|l| l.price.clone()),
            // last_trade_price from /book is market-level, not per-token — omitted per-token
            // to avoid misleading output. Use market-level last_trade_price below.
        }));
    }

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "id": market.id,
            "condition_id": market.condition_id,
            "slug": sanitize_opt_owned(&market.slug),
            "question": sanitize_opt_owned(&market.question),
            "description": sanitize_opt_owned(&market.description),
            "end_date": market.end_date,
            "active": market.active,
            "closed": market.closed,
            "accepting_orders": market.accepting_orders,
            "neg_risk": market.neg_risk,
            "fee_bps": fee_bps,
            "tokens": token_info,
            "volume_24hr": market.volume24hr,
            "volume": market.volume,
            "liquidity": market.liquidity,
            "best_bid": market.best_bid,
            "best_ask": market.best_ask,
            "last_trade_price": market.last_trade_price,
        }
    }))
}
