use clap::Args;
use crate::api::{get_spot_meta, get_all_mids};
use crate::config::info_url;

#[derive(Args)]
pub struct SpotPricesArgs {
    /// Show price for a specific token (e.g. PURR, HYPE).
    /// Omit to list all spot markets.
    #[arg(long)]
    pub token: Option<String>,

    /// Only show canonical markets (filters out non-canonical @N markets with no readable name)
    #[arg(long)]
    pub canonical_only: bool,
}

pub async fn run(args: SpotPricesArgs) -> anyhow::Result<()> {
    let info = info_url();

    let (spot_meta, mids) = tokio::try_join!(get_spot_meta(info), get_all_mids(info))?;

    let empty_vec = vec![];
    let tokens = spot_meta["tokens"].as_array().unwrap_or(&empty_vec);
    let universe = spot_meta["universe"].as_array().unwrap_or(&empty_vec);

    // Build token_index -> token entry map
    let tok_by_idx: std::collections::HashMap<usize, &serde_json::Value> = tokens
        .iter()
        .filter_map(|t| Some((t["index"].as_u64()? as usize, t)))
        .collect();

    // Single token lookup
    if let Some(ref token_name) = args.token {
        let upper = token_name.to_uppercase();
        let token = tokens
            .iter()
            .find(|t| t["name"].as_str().map(|n| n.to_uppercase()) == Some(upper.clone()))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Spot token '{}' not found — run `hyperliquid spot-prices` to list all tokens",
                    token_name
                )
            })?;

        let tok_idx = token["index"].as_u64().unwrap_or(0) as usize;

        let market = universe
            .iter()
            .find(|m| {
                m["tokens"]
                    .as_array()
                    .and_then(|t| t.first())
                    .and_then(|v| v.as_u64())
                    .map(|idx| idx as usize == tok_idx)
                    .unwrap_or(false)
            })
            .ok_or_else(|| anyhow::anyhow!("No spot market found for '{}'", token_name))?;

        let mkt_idx = market["index"].as_u64().unwrap_or(0) as usize;
        // Canonical markets are keyed in allMids by `name` (e.g. "PURR/USDC"); non-canonical
        // markets are keyed by `@<index>`. Prefer the explicit name to handle both cases.
        let market_name = market["name"].as_str().unwrap_or("");
        let fallback_key = format!("@{}", mkt_idx);
        let price = mids
            .get(market_name)
            .or_else(|| mids.get(&fallback_key))
            .and_then(|v| v.as_str())
            .unwrap_or("unavailable");

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "token": token["name"].as_str().unwrap_or(token_name),
                "marketName": market_name,
                "marketIndex": mkt_idx,
                "assetIndex": 10000 + mkt_idx,
                "midPrice": price,
                "szDecimals": token["szDecimals"],
                "isCanonical": market["isCanonical"]
            }))?
        );
        return Ok(());
    }

    // List all markets
    let mut markets = Vec::new();

    for market in universe {
        let is_canonical = market["isCanonical"].as_bool().unwrap_or(false);
        if args.canonical_only && !is_canonical {
            continue;
        }

        let mkt_idx = market["index"].as_u64().unwrap_or(0) as usize;
        let market_name = market["name"].as_str().unwrap_or("");
        let fallback_key = format!("@{}", mkt_idx);
        let price = mids
            .get(market_name)
            .or_else(|| mids.get(&fallback_key))
            .and_then(|v| v.as_str())
            .unwrap_or("0");

        // Resolve base token name
        let base_tok_idx = market["tokens"]
            .as_array()
            .and_then(|t| t.first())
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let base_name = tok_by_idx
            .get(&base_tok_idx)
            .and_then(|t| t["name"].as_str())
            .unwrap_or("?");

        markets.push(serde_json::json!({
            "token": base_name,
            "marketName": market_name,
            "marketIndex": mkt_idx,
            "midPrice": price,
            "isCanonical": is_canonical
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "count": markets.len(),
            "markets": markets
        }))?
    );

    Ok(())
}
