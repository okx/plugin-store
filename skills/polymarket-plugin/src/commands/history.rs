/// `polymarket history` — show trade activity for the active wallet, enriched with
/// win/loss resolution and (for POLY_PROXY mode) deposit/withdrawal transfers.
///
/// Trade activity: Polymarket Data API `/activity` (buys, sells, redeems).
/// Resolution: CLOB API `/markets/{condition_id}` in parallel for each unique market.
/// Transfers: eth_getLogs on USDC.e contract for proxy→EOA and EOA→proxy movements.

use anyhow::{Context, Result};
use reqwest::Client;
use std::collections::HashSet;

pub async fn run(limit: u32, address: Option<&str>) -> Result<()> {
    let client = Client::new();

    // Resolve wallet: proxy wallet in POLY_PROXY mode, else EOA.
    let eoa = crate::onchainos::get_wallet_address().await?;
    let creds = crate::config::load_credentials().ok().flatten();
    let proxy_wallet = creds.as_ref().and_then(|c| {
        if c.mode == crate::config::TradingMode::PolyProxy {
            c.proxy_wallet.clone()
        } else {
            None
        }
    });

    let (wallet_addr, is_proxy_mode) = if let Some(a) = address {
        (a.to_string(), false)  // explicit address → no automatic proxy enrichment
    } else if let Some(ref p) = proxy_wallet {
        (p.clone(), true)
    } else {
        (eoa.clone(), false)
    };

    // ── Fetch trade activity ────────────────────────────────────────────────

    let url = format!(
        "{}/activity?user={}&limit={}&offset=0",
        crate::config::Urls::DATA,
        wallet_addr,
        limit,
    );

    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .context("fetching activity from Data API")?
        .json()
        .await
        .context("parsing activity response")?;

    let mut items: Vec<serde_json::Value> = if resp.is_array() {
        resp.as_array().cloned().unwrap_or_default()
    } else {
        resp["data"].as_array().cloned().unwrap_or_default()
    };

    // ── Batch-resolve market outcomes ───────────────────────────────────────

    let condition_ids: Vec<String> = items
        .iter()
        .filter_map(|item| item["conditionId"].as_str().map(String::from))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let resolutions = if !condition_ids.is_empty() {
        crate::api::get_market_resolutions(&client, &condition_ids).await
    } else {
        std::collections::HashMap::new()
    };

    // Enrich each activity item with a `result` field
    for item in items.iter_mut() {
        let cid = item["conditionId"].as_str().unwrap_or("");
        let outcome_idx = item["outcomeIndex"].as_u64().map(|i| i as u32);

        let result_str = match resolutions.get(cid) {
            Some(Some(winner_idx)) => match outcome_idx {
                Some(bet) if bet == *winner_idx => "WON",
                Some(_) => "LOST",
                None => "RESOLVED",
            },
            Some(None) => "ACTIVE",  // not yet resolved
            None       => "ACTIVE",  // not in resolutions map (lookup failed)
        };

        item["result"] = serde_json::Value::String(result_str.to_string());
    }

    // ── USDC.e deposit/withdrawal transfers (proxy mode only) ───────────────

    let (transfers, transfers_note) = if is_proxy_mode {
        eprintln!("[polymarket] Fetching USDC.e transfer history from Polygon...");
        let t = crate::api::get_usdc_e_transfers(&client, &wallet_addr, &eoa).await;
        let note = if t.is_empty() {
            Some(format!(
                "USDC.e transfer history unavailable from RPC. Full on-chain history: \
                 https://polygonscan.com/token/{}?a={}",
                crate::config::Contracts::USDC_E,
                wallet_addr
            ))
        } else {
            None
        };
        (Some(t), note)
    } else {
        (None, None)
    };

    // ── Output ──────────────────────────────────────────────────────────────

    let mut data = serde_json::json!({
        "wallet": wallet_addr,
        "trade_count": items.len(),
        "trades": items,
    });

    if let Some(t) = transfers {
        data["transfer_count"] = serde_json::json!(t.len());
        data["transfers"] = serde_json::json!(t);
    }
    if let Some(note) = transfers_note {
        data["transfers_note"] = serde_json::json!(note);
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "ok": true, "data": data }))?
    );

    Ok(())
}
