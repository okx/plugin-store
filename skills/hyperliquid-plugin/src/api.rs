use anyhow::Context;
use serde_json::{json, Value};

// ─── HIP-3 (Builder DEX) registry ────────────────────────────────────────────
//
// Hyperliquid HIP-3 introduces "builder-deployed perp DEXs" — independent perp
// venues with separate clearinghouse, oracle, and asset universe. Each has a
// short string name (e.g. "xyz", "flx", "vntl"). The default perp DEX is the
// empty-string-named entry (returned as `null` at index 0 of `perpDexs`).
//
// Asset id math (per Python SDK hyperliquid/info.py):
//   default DEX:  asset_id ∈ [0, ~250)   — coin name unprefixed ("BTC", "ETH")
//   builder DEX i (1-indexed in perpDexs[1:]):  110000 + (i-1) * 10000
//                — coin name prefixed ("xyz:NVDA")
//
// Coin naming:  parse_coin("xyz:CL") → (Some("xyz"), "CL")
//               parse_coin("BTC")    → (None, "BTC")
//
// Registry is fetched once at startup (perpDexs is small, ~9 entries) and cached.

/// Parse a coin string into optional dex prefix + base symbol.
/// "xyz:CL" → (Some("xyz"), "CL"); "BTC" → (None, "BTC").
pub fn parse_coin(coin: &str) -> (Option<String>, String) {
    if let Some((dex, base)) = coin.split_once(':') {
        (Some(dex.to_string()), base.to_string())
    } else {
        (None, coin.to_string())
    }
}

/// Information about one builder DEX (HIP-3 perpDexs entry).
#[derive(Debug, Clone)]
pub struct BuilderDex {
    pub name: String,
    pub full_name: String,
    pub deployer: String,
    pub fee_recipient: Option<String>,
    /// 1-indexed position in `perpDexs[1:]` (skipping the leading null).
    /// Used to compute asset id offset: 110000 + (index - 1) * 10000.
    pub index: usize,
}

impl BuilderDex {
    /// Asset id offset for this DEX's universe. Coin at universe index `j` has
    /// global asset id = `offset + j`.
    pub fn asset_offset(&self) -> usize {
        110_000 + (self.index - 1) * 10_000
    }
}

/// POST /info {"type":"perpDexs"} — returns array of all perp DEXs.
/// Index 0 is `null` (default perp DEX, empty-string name).
/// Indices 1..N are the builder DEXs.
pub async fn fetch_perp_dexs(info_url: &str) -> anyhow::Result<Vec<BuilderDex>> {
    let raw = info_post(info_url, json!({"type": "perpDexs"})).await?;
    let arr = raw.as_array()
        .ok_or_else(|| anyhow::anyhow!("perpDexs response is not an array"))?;
    let mut out = Vec::new();
    for (i, entry) in arr.iter().enumerate() {
        if i == 0 {
            // index 0 is the default DEX (null entry); skip.
            continue;
        }
        if entry.is_null() { continue; }
        let name = entry["name"].as_str()
            .ok_or_else(|| anyhow::anyhow!("perpDexs[{}].name missing", i))?
            .to_string();
        let full_name = entry["fullName"].as_str()
            .unwrap_or(&name).to_string();
        let deployer = entry["deployer"].as_str()
            .unwrap_or("").to_string();
        let fee_recipient = entry["feeRecipient"].as_str().map(|s| s.to_string());
        out.push(BuilderDex { name, full_name, deployer, fee_recipient, index: i });
    }
    Ok(out)
}

/// Resolve a dex name (e.g. "xyz") to the registry entry. None if not found.
pub fn find_dex<'a>(registry: &'a [BuilderDex], name: &str) -> Option<&'a BuilderDex> {
    registry.iter().find(|d| d.name.eq_ignore_ascii_case(name))
}

// ─── HTTP helper ─────────────────────────────────────────────────────────────

/// POST to the Hyperliquid info endpoint.
pub async fn info_post(url: &str, body: Value) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Hyperliquid info HTTP request failed")?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response body")?;

    if !status.is_success() {
        anyhow::bail!("Hyperliquid API error {}: {}", status, text);
    }

    serde_json::from_str(&text).context("Failed to parse Hyperliquid info response as JSON")
}

/// Get all mid prices: POST /info {"type":"allMids"}
/// Returns a map of coin -> mid price string, e.g. {"BTC":"67234.5","ETH":"3456.2",...}
pub async fn get_all_mids(info_url: &str) -> anyhow::Result<Value> {
    info_post(info_url, json!({"type": "allMids"})).await
}

/// HIP-3: Get all mid prices for a specific builder DEX.
/// dex=None -> default DEX (same as get_all_mids); Some("xyz") -> xyz dex mids.
pub async fn get_all_mids_for_dex(info_url: &str, dex: Option<&str>) -> anyhow::Result<Value> {
    let mut body = json!({"type": "allMids"});
    if let Some(d) = dex { body["dex"] = json!(d); }
    info_post(info_url, body).await
}

/// Get clearinghouse state for a user (perp positions, margin summary).
/// POST /info {"type":"clearinghouseState","user":"0x..."}
pub async fn get_clearinghouse_state(info_url: &str, user: &str) -> anyhow::Result<Value> {
    info_post(
        info_url,
        json!({
            "type": "clearinghouseState",
            "user": user
        }),
    )
    .await
}

/// HIP-3: Per-dex clearinghouse state. Each builder DEX has separate margin/positions.
pub async fn get_clearinghouse_state_for_dex(info_url: &str, user: &str, dex: Option<&str>) -> anyhow::Result<Value> {
    let mut body = json!({"type": "clearinghouseState", "user": user});
    if let Some(d) = dex { body["dex"] = json!(d); }
    info_post(info_url, body).await
}

/// Get open orders for a user.
/// POST /info {"type":"openOrders","user":"0x..."}
pub async fn get_open_orders(info_url: &str, user: &str) -> anyhow::Result<Value> {
    info_post(
        info_url,
        json!({
            "type": "openOrders",
            "user": user
        }),
    )
    .await
}

/// HIP-3: Per-dex open orders.
pub async fn get_open_orders_for_dex(info_url: &str, user: &str, dex: Option<&str>) -> anyhow::Result<Value> {
    let mut body = json!({"type": "openOrders", "user": user});
    if let Some(d) = dex { body["dex"] = json!(d); }
    info_post(info_url, body).await
}

/// Get metadata for all perpetual markets (asset index map, etc.).
/// POST /info {"type":"meta"}
pub async fn get_meta(info_url: &str) -> anyhow::Result<Value> {
    info_post(info_url, json!({"type": "meta"})).await
}

/// HIP-3: Get metadata for a specific perp DEX.
/// dex=None -> default DEX (same as get_meta); Some("xyz") -> xyz universe.
pub async fn get_meta_for_dex(info_url: &str, dex: Option<&str>) -> anyhow::Result<Value> {
    let mut body = json!({"type": "meta"});
    if let Some(d) = dex { body["dex"] = json!(d); }
    info_post(info_url, body).await
}

/// HIP-3: meta + per-asset contexts (markPx, prevDayPx, dayNtlVlm, oraclePx, etc.)
/// Returns a 2-element array [meta, asset_ctxs].
/// Used to detect halted markets (markPx == null on weekends/after-hours for equity DEXs).
pub async fn get_meta_and_asset_ctxs_for_dex(info_url: &str, dex: Option<&str>) -> anyhow::Result<Value> {
    let mut body = json!({"type": "metaAndAssetCtxs"});
    if let Some(d) = dex { body["dex"] = json!(d); }
    info_post(info_url, body).await
}

/// HIP-3: Look up the GLOBAL asset id for a coin, taking the dex prefix into account.
/// "BTC"     → (asset_id, sz_decimals) on default DEX
/// "xyz:CL"  → (110029, sz_decimals) on xyz DEX (110000 + universe_idx within xyz)
/// Returns (asset_id, sz_decimals).
pub async fn get_asset_meta_for_coin(
    info_url: &str,
    coin: &str,
    registry: &[BuilderDex],
) -> anyhow::Result<(usize, u32)> {
    let (id, sz, _) = get_asset_meta_with_flags(info_url, coin, registry).await?;
    Ok((id, sz))
}

/// HIP-3: Like `get_asset_meta_for_coin` but ALSO returns the `onlyIsolated` flag.
/// Some HIP-3 markets (xyz:CL / xyz:HOOD / xyz:INTC / etc.) reject cross-margin orders;
/// the order command checks this flag and auto-enables --isolated when true.
pub async fn get_asset_meta_with_flags(
    info_url: &str,
    coin: &str,
    registry: &[BuilderDex],
) -> anyhow::Result<(usize, u32, bool)> {
    let (dex_opt, base) = parse_coin(coin);
    match dex_opt {
        None => {
            // Default DEX path: get asset meta + flag from the universe entry
            let meta = get_meta(info_url).await?;
            let universe = meta["universe"].as_array()
                .ok_or_else(|| anyhow::anyhow!("meta.universe missing"))?;
            let coin_upper = base.to_uppercase();
            for (i, asset) in universe.iter().enumerate() {
                if let Some(name) = asset["name"].as_str() {
                    if name.to_uppercase() == coin_upper {
                        let sz_dec = asset["szDecimals"].as_u64().unwrap_or(4) as u32;
                        let only_isolated = asset["onlyIsolated"].as_bool().unwrap_or(false);
                        return Ok((i, sz_dec, only_isolated));
                    }
                }
            }
            anyhow::bail!("Coin '{}' not found in default DEX universe", coin)
        }
        Some(dex_name) => {
            let dex = find_dex(registry, &dex_name)
                .ok_or_else(|| anyhow::anyhow!(
                    "Unknown DEX '{}'. Run `hyperliquid-plugin dex-list` to see registered builder DEXs.",
                    dex_name))?;
            let meta = get_meta_for_dex(info_url, Some(&dex.name)).await?;
            let universe = meta["universe"].as_array()
                .ok_or_else(|| anyhow::anyhow!("meta.universe missing for DEX {}", dex.name))?;
            let coin_upper = coin.to_uppercase();
            for (i, asset) in universe.iter().enumerate() {
                if let Some(name) = asset["name"].as_str() {
                    if name.to_uppercase() == coin_upper {
                        let sz_dec = asset["szDecimals"].as_u64().unwrap_or(4) as u32;
                        let only_isolated = asset["onlyIsolated"].as_bool().unwrap_or(false);
                        return Ok((dex.asset_offset() + i, sz_dec, only_isolated));
                    }
                }
            }
            anyhow::bail!("Coin '{}' not found in {} DEX universe", coin, dex.name)
        }
    }
}

/// Look up the asset index for a coin symbol from meta.
/// Returns None if the coin is not found.
pub async fn get_asset_index(info_url: &str, coin: &str) -> anyhow::Result<usize> {
    let (idx, _) = get_asset_meta(info_url, coin).await?;
    Ok(idx)
}

/// Look up the asset index AND szDecimals for a coin symbol from meta.
pub async fn get_asset_meta(info_url: &str, coin: &str) -> anyhow::Result<(usize, u32)> {
    let meta = get_meta(info_url).await?;
    let universe = meta["universe"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("meta.universe missing or not an array"))?;

    let coin_upper = coin.to_uppercase();
    for (i, asset) in universe.iter().enumerate() {
        if let Some(name) = asset["name"].as_str() {
            if name.to_uppercase() == coin_upper {
                let sz_dec = asset["szDecimals"].as_u64().unwrap_or(4) as u32;
                return Ok((i, sz_dec));
            }
        }
    }
    anyhow::bail!("Coin '{}' not found in Hyperliquid universe", coin)
}

/// Get spot token + market metadata.
/// POST /info {"type":"spotMeta"}
pub async fn get_spot_meta(info_url: &str) -> anyhow::Result<Value> {
    info_post(info_url, json!({"type": "spotMeta"})).await
}

/// Get spot clearinghouse state for a user (spot balances).
/// POST /info {"type":"spotClearinghouseState","user":"0x..."}
pub async fn get_spot_clearinghouse_state(info_url: &str, user: &str) -> anyhow::Result<Value> {
    info_post(
        info_url,
        json!({
            "type": "spotClearinghouseState",
            "user": user
        }),
    )
    .await
}

/// Look up the spot asset index, market index, AND szDecimals for a token symbol.
/// Returns (asset_index, market_index, sz_decimals).
/// Spot asset index on HL = 10000 + spot market index.
pub async fn get_spot_asset_meta(info_url: &str, coin: &str) -> anyhow::Result<(usize, usize, u32)> {
    let meta = get_spot_meta(info_url).await?;
    let tokens = meta["tokens"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("spotMeta.tokens missing"))?;
    let universe = meta["universe"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("spotMeta.universe missing"))?;

    let coin_upper = coin.to_uppercase();

    // Find token index by name
    let tok_idx = tokens
        .iter()
        .find(|t| t["name"].as_str().map(|n| n.to_uppercase()) == Some(coin_upper.clone()))
        .and_then(|t| t["index"].as_u64())
        .ok_or_else(|| anyhow::anyhow!("Spot token '{}' not found", coin))? as usize;

    // Find market that has this token as base (first token in tokens array)
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
        .ok_or_else(|| anyhow::anyhow!("No spot market for '{}'", coin))?;

    let mkt_idx = market["index"].as_u64().unwrap_or(0) as usize;
    let sz_decimals = tokens
        .iter()
        .find(|t| t["index"].as_u64().map(|i| i as usize) == Some(tok_idx))
        .and_then(|t| t["szDecimals"].as_u64())
        .unwrap_or(2) as u32;

    // Returns (asset_index, market_index, sz_decimals)
    // asset_index = 10000 + market_index (used in HL order actions for spot)
    Ok((10000 + mkt_idx, mkt_idx, sz_decimals))
}
