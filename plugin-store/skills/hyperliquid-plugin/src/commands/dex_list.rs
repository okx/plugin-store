use clap::Args;
use serde_json::{json, Value};

use crate::api::{
    fetch_perp_dexs, get_clearinghouse_state, get_clearinghouse_state_for_dex,
    get_meta_and_asset_ctxs_for_dex,
};
use crate::config::{info_url, ARBITRUM_CHAIN_ID};
use crate::onchainos::resolve_wallet;

/// List all Hyperliquid perp DEXs (default + HIP-3 builder DEXs).
///
/// Builder DEXs are independent perp venues with separate clearinghouse, oracle,
/// and asset universe. Each has a short string name (e.g. "xyz", "flx", "vntl").
/// The `xyz` DEX hosts RWAs (BRENTOIL, GOLD, NVDA, TSLA, SP500, EUR, JPY, etc.).
///
/// For each DEX, this command shows the user's USDC balance, asset count, and
/// 24h volume snapshot. Helps decide which DEX to fund (via `dex-transfer`)
/// before placing orders on builder-deployed markets.
#[derive(Args)]
pub struct DexListArgs {
    /// Wallet address to query for per-DEX balance (default: connected onchainos signing wallet)
    #[arg(long)]
    pub address: Option<String>,
    /// Show detailed breakdown including all asset names per DEX (verbose; default off)
    #[arg(long)]
    pub verbose: bool,
}

pub async fn run(args: DexListArgs) -> anyhow::Result<()> {
    let info_url = info_url();

    // Resolve user wallet (same address used for default-DEX clearinghouse lookup)
    let wallet = match args.address {
        Some(a) => a,
        None => match resolve_wallet(ARBITRUM_CHAIN_ID) {
            Ok(addr) => addr,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("Could not resolve signing wallet: {:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run `onchainos wallet addresses` to verify login, or pass --address explicitly.",
                ));
                return Ok(());
            }
        },
    };

    // 1. Fetch the registry of builder DEXs
    let registry = match fetch_perp_dexs(info_url).await {
        Ok(r) => r,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("perpDexs fetch failed: {:#}", e),
                "RPC_ERROR",
                "Hyperliquid info endpoint may be limited; retry shortly.",
            ));
            return Ok(());
        }
    };

    // 2. For default DEX + each builder DEX, parallel fetch:
    //    - clearinghouseState (user's USDC balance + position count)
    //    - metaAndAssetCtxs (asset count + total 24h ntl volume)
    let default_state_fut = get_clearinghouse_state(info_url, &wallet);
    let default_meta_fut = get_meta_and_asset_ctxs_for_dex(info_url, None);

    // Builder DEX futures
    let builder_futs: Vec<_> = registry.iter().map(|d| {
        let wallet = wallet.clone();
        let dex_name = d.name.clone();
        async move {
            let state_fut = get_clearinghouse_state_for_dex(info_url, &wallet, Some(&dex_name));
            let meta_fut = get_meta_and_asset_ctxs_for_dex(info_url, Some(&dex_name));
            let (state, meta) = tokio::join!(state_fut, meta_fut);
            (dex_name, state.ok(), meta.ok())
        }
    }).collect();

    let (default_state, default_meta, builder_results) = tokio::join!(
        default_state_fut,
        default_meta_fut,
        futures::future::join_all(builder_futs)
    );

    // 3. Build response
    let default_summary = summarize_dex(
        "default", "Default Hyperliquid Perp",
        0, // default DEX has offset 0
        default_state.ok(),
        default_meta.ok(),
        args.verbose,
    );

    let mut builder_summaries: Vec<Value> = Vec::new();
    for ((dex_name, state, meta), info) in builder_results.into_iter().zip(registry.iter()) {
        builder_summaries.push(summarize_dex(
            &dex_name, &info.full_name, info.asset_offset(),
            state, meta, args.verbose,
        ));
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "wallet": wallet,
        "default_dex": default_summary,
        "builder_dexs_count": registry.len(),
        "builder_dexs": builder_summaries,
        "note": "HIP-3 builder DEXs have SEPARATE margin pools — your USDC on the default DEX is NOT shared with builder DEXs. Use `dex-transfer` to move USDC between DEXs before placing orders on builder-deployed markets.",
    }))?);
    Ok(())
}

fn summarize_dex(
    name: &str,
    full_name: &str,
    asset_offset: usize,
    state: Option<Value>,
    meta_pair: Option<Value>,
    verbose: bool,
) -> Value {
    let (account_value, withdrawable, position_count) = match &state {
        Some(s) => {
            let av = s["marginSummary"]["accountValue"].as_str().unwrap_or("0").to_string();
            let wd = s["withdrawable"].as_str().unwrap_or("0").to_string();
            let positions = s["assetPositions"].as_array().map(|a| a.len()).unwrap_or(0);
            (av, wd, positions)
        }
        None => ("?".to_string(), "?".to_string(), 0),
    };

    let (asset_count, halted_count, total_ntl_24h, asset_names) = match &meta_pair {
        Some(p) => {
            let arr = p.as_array();
            if let Some(a) = arr {
                if a.len() >= 2 {
                    let universe = a[0]["universe"].as_array();
                    let ctxs = a[1].as_array();
                    let count = universe.map(|u| u.len()).unwrap_or(0);
                    let mut halted = 0usize;
                    let mut total_vol = 0.0f64;
                    if let Some(c) = ctxs {
                        for ctx in c {
                            if ctx["markPx"].is_null() { halted += 1; }
                            if let Some(v) = ctx["dayNtlVlm"].as_str() {
                                total_vol += v.parse::<f64>().unwrap_or(0.0);
                            }
                        }
                    }
                    let names: Vec<String> = if verbose {
                        universe.map(|u| u.iter()
                            .filter_map(|a| a["name"].as_str().map(|s| s.to_string()))
                            .collect()).unwrap_or_default()
                    } else { Vec::new() };
                    (count, halted, total_vol, names)
                } else {
                    (0, 0, 0.0, Vec::new())
                }
            } else {
                (0, 0, 0.0, Vec::new())
            }
        }
        None => (0, 0, 0.0, Vec::new()),
    };

    let mut entry = json!({
        "name": name,
        "full_name": full_name,
        "asset_offset": asset_offset,
        "asset_count": asset_count,
        "halted_count": halted_count,
        "user_account_value_usd": account_value,
        "user_withdrawable_usd": withdrawable,
        "user_position_count": position_count,
        "total_ntl_volume_24h_usd": format!("{:.0}", total_ntl_24h),
    });
    if verbose {
        entry["assets"] = json!(asset_names);
    }
    entry
}
