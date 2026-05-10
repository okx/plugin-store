use clap::Args;
use serde_json::{json, Value};
use std::collections::HashSet;

use crate::api::{
    fetch_perp_dexs, find_dex, get_all_mids, get_meta_and_asset_ctxs_for_dex, get_spot_meta,
    BuilderDex,
};
use crate::config::info_url;

/// List tradeable markets across Hyperliquid (default perp DEX, HIP-3 builder DEXs, or spot).
///
/// `--type` is a semantic shortcut over the underlying venue/product split:
///   crypto -> perp on the default DEX (~230 crypto perps)
///   tradfi -> perps on builder DEXs, EXCLUDING crypto duplicates (xyz:BTC/ETH/SOL filtered)
///   hip3   -> perps on builder DEXs, INCLUDING all crypto duplicates
///   spot   -> spot markets (HYPE, PURR, etc.)
///
/// Examples:
///   # Top-30 most-traded crypto perps (default)
///   hyperliquid-plugin markets
///
///   # All TradFi RWAs (commodities / equities / indices / FX) sorted by 24h vol
///   hyperliquid-plugin markets --type tradfi
///
///   # Only large TradFi markets ($10M+ daily vol)
///   hyperliquid-plugin markets --type tradfi --min-vol 10000000
///
///   # Specific symbol on a builder DEX
///   hyperliquid-plugin markets --coin xyz:CL
///
///   # All markets on a specific builder DEX
///   hyperliquid-plugin markets --dex flx
#[derive(Args)]
pub struct MarketsArgs {
    /// Semantic shortcut: crypto | tradfi | hip3 | spot | outcome. Default: crypto.
    /// `outcome` (also: hip4) lists HIP-4 binary YES/NO outcome contracts.
    /// Mutually exclusive with --dex.
    #[arg(long)]
    pub r#type: Option<String>,

    /// Specific perp DEX name (default | xyz | flx | vntl | cash | km | hyna | para | abcd).
    /// Mutually exclusive with --type.
    #[arg(long)]
    pub dex: Option<String>,

    /// Look up a single symbol (e.g. BTC, xyz:CL, HYPE for spot).
    /// When set, all other filters are ignored.
    #[arg(long)]
    pub coin: Option<String>,

    /// Filter: minimum 24h notional volume in USD (perp only)
    #[arg(long)]
    pub min_vol: Option<f64>,

    /// Filter: minimum max leverage (perp only)
    #[arg(long)]
    pub max_leverage: Option<u32>,

    /// Filter: only show markets with onlyIsolated=true (perp only)
    #[arg(long)]
    pub only_isolated: bool,

    /// Filter: hide markets that are currently halted (markPx == null, perp only)
    #[arg(long)]
    pub hide_halted: bool,

    /// Sort key: vol (default) | leverage | symbol
    #[arg(long, default_value = "vol")]
    pub sort: String,

    /// Max rows to return. 0 = no limit. Default: 30.
    #[arg(long, default_value_t = 30)]
    pub limit: usize,
}

pub async fn run(args: MarketsArgs) -> anyhow::Result<()> {
    let info = info_url();

    if args.r#type.is_some() && args.dex.is_some() {
        println!("{}", super::error_response(
            "--type and --dex are mutually exclusive",
            "INVALID_ARGUMENT",
            "Pick one: --type for semantic preset (crypto/tradfi/hip3/spot/outcome), --dex for a specific perp DEX name.",
        ));
        return Ok(());
    }

    // Resolve effective type/dex
    let mode = resolve_mode(args.r#type.as_deref(), args.dex.as_deref());
    let mode = match mode {
        Ok(m) => m,
        Err(e) => {
            println!("{}", super::error_response(&e, "INVALID_TYPE",
                "Valid --type values: crypto, tradfi, hip3, spot, outcome."));
            return Ok(());
        }
    };

    // Single-coin lookup short-circuits filters
    if let Some(coin) = args.coin.as_deref() {
        return lookup_single(info, coin, &mode).await;
    }

    match mode {
        Mode::Spot => list_spot(info, &args).await,
        Mode::PerpDefault => list_perp_default(info, &args).await,
        Mode::PerpBuilders { dedupe_crypto } => {
            list_perp_builders(info, &args, dedupe_crypto).await
        }
        Mode::PerpDex(dex_name) => list_perp_single_dex(info, &args, &dex_name).await,
        Mode::Outcome => list_outcome(info, &args).await,
    }
}

#[derive(Debug, Clone)]
enum Mode {
    PerpDefault,
    PerpBuilders { dedupe_crypto: bool },
    PerpDex(String),
    Spot,
    Outcome,
}

fn resolve_mode(ty: Option<&str>, dex: Option<&str>) -> Result<Mode, String> {
    if let Some(d) = dex {
        if d.eq_ignore_ascii_case("default") || d.is_empty() {
            return Ok(Mode::PerpDefault);
        }
        return Ok(Mode::PerpDex(d.to_string()));
    }
    let ty = ty.unwrap_or("crypto").to_lowercase();
    match ty.as_str() {
        "crypto" => Ok(Mode::PerpDefault),
        "tradfi" => Ok(Mode::PerpBuilders { dedupe_crypto: true }),
        "hip3" | "hip-3" => Ok(Mode::PerpBuilders { dedupe_crypto: false }),
        "spot" => Ok(Mode::Spot),
        "outcome" | "outcomes" | "hip4" | "hip-4" | "prediction" => Ok(Mode::Outcome),
        other => Err(format!("Unknown --type '{}'", other)),
    }
}

// ---------- Single-coin lookup ----------

async fn lookup_single(info: &str, coin: &str, mode: &Mode) -> anyhow::Result<()> {
    // Builder DEX market with explicit prefix ("xyz:CL") always wins over --type:
    // user gave a fully-qualified coin, route directly.
    if coin.contains(':') {
        let (dex, _) = crate::api::parse_coin(coin);
        let dex = dex.unwrap_or_default();
        return lookup_perp(info, coin, Some(&dex)).await;
    }

    // No prefix → search where --type / --dex tells us to look. Previous behavior
    // (always default-perp then spot) silently dropped builder-DEX matches when
    // user passed `--type tradfi --coin NVDA`.
    match mode {
        Mode::Spot => lookup_spot(info, coin).await,
        Mode::PerpDefault => {
            // Default DEX → fallback to spot for backward compat
            let default_meta = get_meta_and_asset_ctxs_for_dex(info, None).await;
            if let Ok(meta) = default_meta {
                if let Some(entry) = find_perp_market(&meta, coin, None) {
                    print_perp_single("default", entry);
                    return Ok(());
                }
            }
            lookup_spot(info, coin).await
        }
        Mode::PerpDex(dex_name) => lookup_perp(info, coin, Some(dex_name)).await,
        Mode::PerpBuilders { dedupe_crypto } => {
            lookup_across_builders(info, coin, *dedupe_crypto).await
        }
        Mode::Outcome => {
            // Outcome markets are looked up by their full outcome id, not by
            // bare token names. Fall back to existing single-asset error path.
            println!("{}", super::error_response(
                &format!("Outcome lookup by --coin '{}' not supported; use `outcome-list` to discover markets", coin),
                "MARKET_NOT_FOUND",
                "Use `hyperliquid-plugin outcome-list` to enumerate outcomes, then trade with `outcome-buy --outcome <id>`.",
            ));
            Ok(())
        }
    }
}

/// Search across all HIP-3 builder DEXs in parallel for the first matching coin.
/// `dedupe_crypto = true` (--type tradfi) skips a builder match if the bare
/// symbol also exists on default DEX (e.g. xyz:BTC dedup'd because BTC is on
/// default). When false (--type hip3) every builder match counts.
async fn lookup_across_builders(
    info: &str,
    coin: &str,
    dedupe_crypto: bool,
) -> anyhow::Result<()> {
    let registry = match fetch_perp_dexs(info).await {
        Ok(r) => r,
        Err(e) => return print_api_err(&e),
    };

    // crypto symbol set on default DEX, used only when dedupe_crypto is on.
    let crypto_set: HashSet<String> = if dedupe_crypto {
        get_meta_and_asset_ctxs_for_dex(info, None)
            .await
            .ok()
            .as_ref()
            .and_then(|m| m.as_array())
            .and_then(|a| a.first())
            .and_then(|m| m["universe"].as_array())
            .map(|u| {
                u.iter()
                    .filter_map(|x| x["name"].as_str())
                    .map(|s| s.to_uppercase())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        HashSet::new()
    };

    if dedupe_crypto && crypto_set.contains(&coin.to_uppercase()) {
        // user said --type tradfi --coin BTC: BTC is a default-DEX crypto, not RWA
        println!("{}", super::error_response(
            &format!("'{}' is a crypto perp on default DEX, not a tradfi/RWA market. Use --type crypto (or no --type) to look up default-DEX coins.", coin),
            "MARKET_NOT_FOUND",
            "Run without --type for default DEX, or specify --coin <dex>:<symbol> to force a builder DEX lookup.",
        ));
        return Ok(());
    }

    // Parallel fetch every builder DEX's meta, take first match.
    let futs: Vec<_> = registry
        .iter()
        .map(|d: &BuilderDex| {
            let name = d.name.clone();
            async move {
                let meta = get_meta_and_asset_ctxs_for_dex(info, Some(&name))
                    .await
                    .ok();
                (name, meta)
            }
        })
        .collect();
    let results = futures::future::join_all(futs).await;

    for (dex_name, meta_opt) in results {
        let Some(meta) = meta_opt else { continue };
        if let Some(entry) = find_perp_market(&meta, coin, Some(dex_name.clone())) {
            print_perp_single(&dex_name, entry);
            return Ok(());
        }
    }

    let preset = if dedupe_crypto { "tradfi" } else { "hip3" };
    println!("{}", super::error_response(
        &format!("Symbol '{}' not found on any builder DEX (--type {})", coin, preset),
        "MARKET_NOT_FOUND",
        "Run `hyperliquid-plugin markets --type tradfi` (no --coin) to list all RWA/equity markets, or use `--coin <dex>:<symbol>` if you know the DEX prefix.",
    ));
    Ok(())
}

async fn lookup_perp(info: &str, coin: &str, dex_opt: Option<&str>) -> anyhow::Result<()> {
    let dex = dex_opt.filter(|d| !d.is_empty());
    let meta = match get_meta_and_asset_ctxs_for_dex(info, dex).await {
        Ok(m) => m,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("Failed to fetch metaAndAssetCtxs: {:#}", e),
                "API_ERROR", "Hyperliquid info endpoint may be limited; retry shortly.",
            ));
            return Ok(());
        }
    };
    let dex_label = dex.unwrap_or("default").to_string();
    if let Some(entry) = find_perp_market(&meta, coin, dex.map(|s| s.to_string())) {
        print_perp_single(&dex_label, entry);
    } else {
        println!("{}", super::error_response(
            &format!("Market '{}' not found on dex '{}'", coin, dex_label),
            "MARKET_NOT_FOUND",
            "Run `hyperliquid-plugin markets --type tradfi` (or --dex <name>) to list available symbols.",
        ));
    }
    Ok(())
}

async fn lookup_spot(info: &str, token: &str) -> anyhow::Result<()> {
    let upper = token.to_uppercase();
    let (spot_meta, mids) = match tokio::try_join!(get_spot_meta(info), get_all_mids(info)) {
        Ok(p) => p,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("Failed to fetch spot meta: {:#}", e),
                "API_ERROR", "Hyperliquid info endpoint may be limited; retry shortly.",
            ));
            return Ok(());
        }
    };
    let empty = vec![];
    let tokens = spot_meta["tokens"].as_array().unwrap_or(&empty);
    let universe = spot_meta["universe"].as_array().unwrap_or(&empty);

    let token_entry = tokens.iter().find(|t|
        t["name"].as_str().map(|s| s.eq_ignore_ascii_case(&upper)).unwrap_or(false));
    let Some(t) = token_entry else {
        println!("{}", super::error_response(
            &format!("Spot token '{}' not found", token),
            "MARKET_NOT_FOUND",
            "Run `hyperliquid-plugin markets --type spot` to list spot tokens.",
        ));
        return Ok(());
    };
    let tok_idx = t["index"].as_u64().unwrap_or(0) as usize;
    let market = universe.iter().find(|m|
        m["tokens"].as_array().and_then(|a| a.first()).and_then(|v| v.as_u64())
            .map(|i| i as usize == tok_idx).unwrap_or(false));
    let Some(m) = market else {
        println!("{}", super::error_response(
            &format!("No spot market for token '{}'", token),
            "MARKET_NOT_FOUND", "",
        ));
        return Ok(());
    };
    let mkt_idx = m["index"].as_u64().unwrap_or(0) as usize;
    let market_name = m["name"].as_str().unwrap_or("");
    // Canonical markets keyed by `name` (e.g. "PURR/USDC"); non-canonical by `@<index>`.
    let fallback_key = format!("@{}", mkt_idx);
    let price = mids
        .get(market_name)
        .or_else(|| mids.get(&fallback_key))
        .and_then(|v| v.as_str())
        .unwrap_or("0");
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "type": "spot",
        "market": {
            "symbol": t["name"].as_str().unwrap_or(token),
            "market_name": market_name,
            "market_index": mkt_idx,
            "mid_px": price,
            "sz_decimals": t["szDecimals"],
            "is_canonical": m["isCanonical"].as_bool().unwrap_or(false),
        }
    })).unwrap());
    Ok(())
}

fn find_perp_market(meta: &Value, coin: &str, dex_prefix: Option<String>) -> Option<Value> {
    let arr = meta.as_array()?;
    if arr.len() < 2 { return None; }
    let universe = arr[0]["universe"].as_array()?;
    let ctxs = arr[1].as_array()?;

    // For builder DEXs the universe entries already contain the prefix in `name`
    // (e.g. "xyz:CL"). For default DEX, names are bare ("BTC").
    let target = if let Some(p) = &dex_prefix {
        if coin.contains(':') { coin.to_string() } else { format!("{}:{}", p, coin) }
    } else {
        // Default DEX: strip any leading "default:" if user added it
        coin.trim_start_matches("default:").to_string()
    };

    for (i, u) in universe.iter().enumerate() {
        let name = u["name"].as_str().unwrap_or("");
        if name.eq_ignore_ascii_case(&target) {
            let ctx = ctxs.get(i).cloned().unwrap_or(Value::Null);
            return Some(build_perp_entry(u, &ctx));
        }
    }
    None
}

fn print_perp_single(dex_label: &str, mut entry: Value) {
    if let Some(obj) = entry.as_object_mut() { obj.remove("day_volume_usd_num"); }
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "type": "perp",
        "dex": dex_label,
        "market": entry,
    })).unwrap());
}

// ---------- List variants ----------

async fn list_perp_default(info: &str, args: &MarketsArgs) -> anyhow::Result<()> {
    let meta = match get_meta_and_asset_ctxs_for_dex(info, None).await {
        Ok(m) => m,
        Err(e) => return print_api_err(&e),
    };
    let entries = build_perp_entries(&meta, "default");
    emit_perp_list(args, "crypto", "default", entries);
    Ok(())
}

async fn list_perp_single_dex(info: &str, args: &MarketsArgs, dex_name: &str) -> anyhow::Result<()> {
    let registry = match fetch_perp_dexs(info).await {
        Ok(r) => r,
        Err(e) => return print_api_err(&e),
    };
    if find_dex(&registry, dex_name).is_none() {
        let known: Vec<String> = registry.iter().map(|d| d.name.clone()).collect();
        println!("{}", super::error_response(
            &format!("Unknown perp DEX '{}'. Known: default, {}", dex_name, known.join(", ")),
            "INVALID_DEX",
            "Run `hyperliquid-plugin dex-list` to see all registered builder DEXs.",
        ));
        return Ok(());
    }
    let meta = match get_meta_and_asset_ctxs_for_dex(info, Some(dex_name)).await {
        Ok(m) => m,
        Err(e) => return print_api_err(&e),
    };
    let entries = build_perp_entries(&meta, dex_name);
    let preset = if dex_name == "default" { "crypto" } else { "hip3" };
    emit_perp_list(args, preset, dex_name, entries);
    Ok(())
}

async fn list_perp_builders(info: &str, args: &MarketsArgs, dedupe_crypto: bool) -> anyhow::Result<()> {
    let registry = match fetch_perp_dexs(info).await {
        Ok(r) => r,
        Err(e) => return print_api_err(&e),
    };

    // Fetch default-DEX universe for crypto-dedupe (only if needed)
    let default_meta_fut = async {
        if dedupe_crypto {
            get_meta_and_asset_ctxs_for_dex(info, None).await.ok()
        } else {
            None
        }
    };

    let builder_futs: Vec<_> = registry.iter().map(|d: &BuilderDex| {
        let name = d.name.clone();
        async move {
            let meta = get_meta_and_asset_ctxs_for_dex(info, Some(&name)).await.ok();
            (name, meta)
        }
    }).collect();

    let (default_meta, builder_results) = tokio::join!(
        default_meta_fut,
        futures::future::join_all(builder_futs)
    );

    // Build crypto-symbol set (bare names from default DEX universe)
    let crypto_set: HashSet<String> = if dedupe_crypto {
        default_meta.as_ref()
            .and_then(|m| m.as_array())
            .and_then(|a| a.first())
            .and_then(|m| m["universe"].as_array())
            .map(|u| u.iter()
                .filter_map(|x| x["name"].as_str())
                .map(|s| s.to_uppercase())
                .collect())
            .unwrap_or_default()
    } else {
        HashSet::new()
    };

    let mut all_entries = Vec::new();
    for (dex_name, meta_opt) in builder_results {
        let Some(meta) = meta_opt else { continue; };
        let entries = build_perp_entries(&meta, &dex_name);
        for e in entries {
            if dedupe_crypto {
                let bare = e["symbol"].as_str().unwrap_or("")
                    .splitn(2, ':').nth(1).unwrap_or("").to_uppercase();
                if crypto_set.contains(&bare) { continue; }
            }
            all_entries.push(e);
        }
    }

    let preset = if dedupe_crypto { "tradfi" } else { "hip3" };
    emit_perp_list(args, preset, "(builder DEXs)", all_entries);
    Ok(())
}

async fn list_spot(info: &str, args: &MarketsArgs) -> anyhow::Result<()> {
    let (spot_meta, mids) = match tokio::try_join!(get_spot_meta(info), get_all_mids(info)) {
        Ok(p) => p,
        Err(e) => return print_api_err(&e),
    };
    let empty = vec![];
    let tokens = spot_meta["tokens"].as_array().unwrap_or(&empty);
    let universe = spot_meta["universe"].as_array().unwrap_or(&empty);
    let tok_by_idx: std::collections::HashMap<usize, &Value> = tokens.iter()
        .filter_map(|t| Some((t["index"].as_u64()? as usize, t))).collect();

    let mut markets: Vec<Value> = Vec::new();
    for m in universe {
        let mkt_idx = m["index"].as_u64().unwrap_or(0) as usize;
        let market_name = m["name"].as_str().unwrap_or("");
        let fallback_key = format!("@{}", mkt_idx);
        let price = mids
            .get(market_name)
            .or_else(|| mids.get(&fallback_key))
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        let base_idx = m["tokens"].as_array().and_then(|a| a.first()).and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let base = tok_by_idx.get(&base_idx);
        let symbol = base.and_then(|t| t["name"].as_str()).unwrap_or("?");
        let sz_decimals = base.and_then(|t| t["szDecimals"].as_u64()).unwrap_or(0);
        markets.push(json!({
            "symbol": symbol,
            "market_name": market_name,
            "market_index": mkt_idx,
            "mid_px": price,
            "sz_decimals": sz_decimals,
            "is_canonical": m["isCanonical"].as_bool().unwrap_or(false),
        }));
    }

    // Sort: canonical first, then by symbol
    markets.sort_by(|a, b| {
        let ac = a["is_canonical"].as_bool().unwrap_or(false);
        let bc = b["is_canonical"].as_bool().unwrap_or(false);
        bc.cmp(&ac)
            .then_with(|| a["symbol"].as_str().unwrap_or("").cmp(b["symbol"].as_str().unwrap_or("")))
    });

    let total = markets.len();
    let shown = if args.limit == 0 { markets } else { markets.into_iter().take(args.limit).collect() };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "type": "spot",
        "count_total": total,
        "count_shown": shown.len(),
        "markets": shown,
    })).unwrap());
    Ok(())
}

// ---------- Helpers ----------

fn build_perp_entries(meta: &Value, dex_name: &str) -> Vec<Value> {
    let Some(arr) = meta.as_array() else { return Vec::new(); };
    if arr.len() < 2 { return Vec::new(); }
    let Some(universe) = arr[0]["universe"].as_array() else { return Vec::new(); };
    let ctxs = arr[1].as_array();

    let mut out = Vec::with_capacity(universe.len());
    for (i, u) in universe.iter().enumerate() {
        let ctx = ctxs.and_then(|c| c.get(i)).cloned().unwrap_or(Value::Null);
        let mut entry = build_perp_entry(u, &ctx);
        entry["dex"] = json!(dex_name);
        out.push(entry);
    }
    out
}

fn build_perp_entry(u: &Value, ctx: &Value) -> Value {
    let symbol = u["name"].as_str().unwrap_or("?").to_string();
    let mark_px = ctx["markPx"].as_str().map(|s| s.to_string());
    let mid_px = ctx["midPx"].as_str().map(|s| s.to_string());
    let day_vol = ctx["dayNtlVlm"].as_str()
        .and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let max_lev = u["maxLeverage"].as_u64().unwrap_or(0) as u32;
    let only_iso = u["onlyIsolated"].as_bool().unwrap_or(false);
    let sz_dec = u["szDecimals"].as_u64().unwrap_or(0) as u32;
    let is_delisted = u["isDelisted"].as_bool().unwrap_or(false);
    let is_halted = mark_px.is_none();

    json!({
        "symbol": symbol,
        "mark_px": mark_px,
        "mid_px": mid_px,
        "day_volume_usd": format!("{:.0}", day_vol),
        "day_volume_usd_num": day_vol,
        "max_leverage": max_lev,
        "only_isolated": only_iso,
        "sz_decimals": sz_dec,
        "is_halted": is_halted,
        "is_delisted": is_delisted,
    })
}

fn emit_perp_list(args: &MarketsArgs, preset: &str, dex_label: &str, mut entries: Vec<Value>) {
    let total_before = entries.len();

    // Apply filters
    if let Some(min_vol) = args.min_vol {
        entries.retain(|e| e["day_volume_usd_num"].as_f64().unwrap_or(0.0) >= min_vol);
    }
    if let Some(min_lev) = args.max_leverage {
        entries.retain(|e| e["max_leverage"].as_u64().unwrap_or(0) >= min_lev as u64);
    }
    if args.only_isolated {
        entries.retain(|e| e["only_isolated"].as_bool().unwrap_or(false));
    }
    if args.hide_halted {
        entries.retain(|e| !e["is_halted"].as_bool().unwrap_or(false));
    }
    // Always hide delisted markets from list view
    entries.retain(|e| !e["is_delisted"].as_bool().unwrap_or(false));

    // Sort
    match args.sort.as_str() {
        "leverage" => entries.sort_by(|a, b| {
            b["max_leverage"].as_u64().unwrap_or(0)
                .cmp(&a["max_leverage"].as_u64().unwrap_or(0))
        }),
        "symbol" => entries.sort_by(|a, b| {
            a["symbol"].as_str().unwrap_or("").cmp(b["symbol"].as_str().unwrap_or(""))
        }),
        _ => entries.sort_by(|a, b| {
            b["day_volume_usd_num"].as_f64().unwrap_or(0.0)
                .partial_cmp(&a["day_volume_usd_num"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    let count_filtered = entries.len();
    let shown: Vec<Value> = if args.limit == 0 {
        entries
    } else {
        entries.into_iter().take(args.limit).collect()
    };

    // Strip the numeric helper field from output
    let shown: Vec<Value> = shown.into_iter().map(|mut e| {
        if let Some(obj) = e.as_object_mut() { obj.remove("day_volume_usd_num"); }
        e
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "type": preset,
        "dex": dex_label,
        "count_total": total_before,
        "count_after_filters": count_filtered,
        "count_shown": shown.len(),
        "sort": args.sort,
        "markets": shown,
    })).unwrap());
}

fn print_api_err(e: &anyhow::Error) -> anyhow::Result<()> {
    println!("{}", super::error_response(
        &format!("Failed to fetch market metadata: {:#}", e),
        "API_ERROR",
        "Hyperliquid info endpoint may be limited; retry shortly.",
    ));
    Ok(())
}

/// HIP-4: delegate to the dedicated `outcome-list` command. We honor the user's
/// `--limit` for output cap but ignore other filter flags (--min-vol, --max-leverage,
/// --only-isolated, --hide-halted, --sort) since outcome markets have a totally
/// different shape (fully-collateralized binary contracts, no leverage, no liquidation,
/// USDH-collateralized — none of those filters apply).
async fn list_outcome(_info: &str, args: &MarketsArgs) -> anyhow::Result<()> {
    let oa = crate::commands::outcome_list::OutcomeListArgs {
        recurring_only: false,
        sort: "id".to_string(),
        limit: args.limit,
    };
    crate::commands::outcome_list::run(oa).await
}
