// src/api.rs — Clanker REST API client
use anyhow::Context;
use serde_json::Value;

const CLANKER_API_BASE: &str = "https://clanker.world/api";

// ── API functions ──────────────────────────────────────────────────────────

/// GET /api/tokens — list recently deployed tokens
pub async fn list_tokens(
    page: u32,
    limit: u32,
    sort: &str,
    chain_id: Option<u64>,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let mut params = vec![
        ("page", page.to_string()),
        ("limit", limit.to_string()),
        ("sort", sort.to_string()),
    ];
    if let Some(cid) = chain_id {
        params.push(("chain_id", cid.to_string()));
    }
    let resp = client
        .get(format!("{}/tokens", CLANKER_API_BASE))
        .query(&params)
        .send()
        .await
        .context("list_tokens HTTP request failed")?
        .json::<Value>()
        .await
        .context("list_tokens JSON parse failed")?;
    Ok(resp)
}

/// GET /api/search-creator — search tokens by creator address or Farcaster username
pub async fn search_creator(
    q: &str,
    limit: u32,
    offset: u32,
    sort: &str,
    trusted_only: bool,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let trusted_str = trusted_only.to_string();
    let params = vec![
        ("q", q.to_string()),
        ("limit", limit.to_string()),
        ("offset", offset.to_string()),
        ("sort", sort.to_string()),
        ("trustedOnly", trusted_str),
    ];
    let resp = client
        .get(format!("{}/search-creator", CLANKER_API_BASE))
        .query(&params)
        .send()
        .await
        .context("search_creator HTTP request failed")?
        .json::<Value>()
        .await
        .context("search_creator JSON parse failed")?;
    Ok(resp)
}

