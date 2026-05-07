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

// ─── HIP-4 (Outcome Markets) ─────────────────────────────────────────────────
//
// HIP-4 introduces fully-collateralized binary YES/NO outcome contracts that
// settle within the 0.001..0.999 price range and represent implied probability.
// They live inside the spot subsystem (no separate clearinghouse), so:
//
//   - Outcome positions appear as `spotClearinghouseState.balances` entries
//     under coin string `+<encoding>`
//   - Outcome orderbook / order placement / l2Book / allMids use coin string
//     `#<encoding>` for the same underlying outcome side
//   - Settlement is fully automatic at expiry (oracle posts result; YES holders
//     credit 1 USDH per share, NO holders credit 0 — or vice versa). No claim
//     action required.
//
// Asset id namespace: `100_000_000 + encoding` where `encoding = 10*outcome_id + side`.
// Side is 0 (Yes) or 1 (No).
//
// Trading is denominated in USDH (Hyperliquid native stablecoin), spot token
// at index 360 on mainnet (USDH/USDC pair = market `@230`, ratio ~0.999995).

/// Asset id offset for HIP-4 outcomes.
pub const OUTCOME_ASSET_ID_BASE: u64 = 100_000_000;
/// YES side index per HIP-4 spec.
pub const OUTCOME_SIDE_YES: u8 = 0;
/// NO side index per HIP-4 spec.
pub const OUTCOME_SIDE_NO: u8 = 1;

/// Compute the HIP-4 global asset id for one side of an outcome.
/// asset_id = 100_000_000 + 10 * outcome_id + side
pub fn outcome_asset_id(outcome_id: u32, side: u8) -> u64 {
    debug_assert!(side <= 1, "outcome side must be 0 (YES) or 1 (NO)");
    OUTCOME_ASSET_ID_BASE + 10u64 * outcome_id as u64 + side as u64
}

/// Coin string used for ORDER PLACEMENT, l2Book lookups, and allMids: `#<encoding>`.
/// Distinct from the balance-context form returned by `outcome_balance_coin`.
pub fn outcome_trade_coin(outcome_id: u32, side: u8) -> String {
    debug_assert!(side <= 1);
    format!("#{}", 10 * outcome_id + side as u32)
}

/// Coin string used in `spotClearinghouseState.balances` entries: `+<encoding>`.
/// Distinct from the trading-context form returned by `outcome_trade_coin` —
/// HL deliberately uses two different prefixes so the same underlying side
/// asset can be unambiguously addressed in either context.
pub fn outcome_balance_coin(outcome_id: u32, side: u8) -> String {
    debug_assert!(side <= 1);
    format!("+{}", 10 * outcome_id + side as u32)
}

/// Parse an outcome coin string in EITHER form (`#<encoding>` or `+<encoding>`).
/// Returns (outcome_id, side) on success, None if the string is not a HIP-4
/// encoding or is malformed.
pub fn parse_outcome_coin(coin: &str) -> Option<(u32, u8)> {
    let rest = coin.strip_prefix('#').or_else(|| coin.strip_prefix('+'))?;
    let encoding: u32 = rest.parse().ok()?;
    let side = (encoding % 10) as u8;
    if side > 1 {
        return None;
    }
    let outcome_id = encoding / 10;
    Some((outcome_id, side))
}

/// One outcome (recurring or builder-deployed). Mirrors HL's `outcomeMeta.outcomes[]`.
#[derive(Debug, Clone)]
pub struct OutcomeSpec {
    pub outcome_id: u32,
    pub name: String,
    pub description: String,
    pub side_names: (String, String),
}

impl OutcomeSpec {
    /// Parse a recurring-priceBinary description into structured fields.
    /// e.g. `"class:priceBinary|underlying:BTC|expiry:20260505-0600|targetPrice:79980|period:1d"`
    /// Returns None for non-priceBinary outcomes (e.g. categorical questions).
    pub fn parse_recurring(&self) -> Option<RecurringSpec> {
        if !self.description.starts_with("class:priceBinary|") {
            return None;
        }
        let mut parts: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for kv in self.description.split('|') {
            if let Some((k, v)) = kv.split_once(':') {
                parts.insert(k, v);
            }
        }
        Some(RecurringSpec {
            class: parts.get("class")?.to_string(),
            underlying: parts.get("underlying")?.to_string(),
            expiry: parts.get("expiry")?.to_string(),
            target_price: parts.get("targetPrice")?.parse().ok()?,
            period: parts.get("period")?.to_string(),
        })
    }
}

/// Parsed recurring-priceBinary description.
#[derive(Debug, Clone)]
pub struct RecurringSpec {
    pub class: String,
    pub underlying: String,
    /// e.g. "20260505-0600" (UTC-ish timestamp string in HL's format).
    pub expiry: String,
    pub target_price: f64,
    pub period: String,
}

/// Fetch the HIP-4 outcome universe from `info {"type":"outcomeMeta"}`.
pub async fn fetch_outcome_meta(info_url: &str) -> anyhow::Result<Vec<OutcomeSpec>> {
    let body = json!({"type": "outcomeMeta"});
    let resp = info_post(info_url, body).await?;
    let arr = resp["outcomes"].as_array().cloned().unwrap_or_default();
    let mut out = Vec::with_capacity(arr.len());
    for o in arr {
        let outcome_id = match o["outcome"].as_u64() {
            Some(v) => v as u32,
            None => continue,
        };
        let name = o["name"].as_str().unwrap_or("").to_string();
        let description = o["description"].as_str().unwrap_or("").to_string();
        let side_names = {
            let arr = o["sideSpecs"].as_array();
            let yes = arr
                .and_then(|a| a.first())
                .and_then(|s| s["name"].as_str())
                .unwrap_or("Yes")
                .to_string();
            let no = arr
                .and_then(|a| a.get(1))
                .and_then(|s| s["name"].as_str())
                .unwrap_or("No")
                .to_string();
            (yes, no)
        };
        out.push(OutcomeSpec {
            outcome_id,
            name,
            description,
            side_names,
        });
    }
    Ok(out)
}
