use clap::Args;
use crate::api::{
    fetch_perp_dexs, get_asset_meta_for_coin, get_meta, get_open_orders_for_dex, parse_coin,
};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_cancel_action, build_batch_cancel_action, submit_exchange_request};

#[derive(Args)]
pub struct CancelArgs {
    /// Cancel a specific order by ID. Also requires --coin.
    #[arg(long)]
    pub order_id: Option<u64>,

    /// Coin symbol (e.g. BTC, ETH).
    /// With --order-id: required to resolve asset index.
    /// Without --order-id: cancels ALL open orders for this coin.
    #[arg(long)]
    pub coin: Option<String>,

    /// Cancel ALL open orders across all coins.
    #[arg(long, conflicts_with_all = ["order_id", "coin"])]
    pub all: bool,

    /// Dry run — preview cancel payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit the cancellation (without this flag, prints a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: CancelArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();
    let nonce = now_ms();

    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
            return Ok(());
        }
    };

    // ── Determine which orders to cancel ──────────────────────────────────────

    // Case 1: single order by ID
    if let Some(oid) = args.order_id {
        let raw_coin = match args.coin.as_deref() {
            Some(c) => c,
            None => {
                println!("{}", super::error_response("--coin is required when using --order-id", "INVALID_ARGUMENT", "Provide --coin <SYMBOL> alongside --order-id (HIP-3: pass full prefixed name e.g. xyz:CL)."));
                return Ok(());
            }
        };
        // HIP-3: parse dex prefix; coin keeps full prefixed form for builder DEX
        let (dex_opt, _) = parse_coin(raw_coin);
        let coin = if dex_opt.is_some() {
            let (d, b) = parse_coin(raw_coin);
            format!("{}:{}", d.unwrap(), b.to_uppercase())
        } else {
            normalize_coin(raw_coin)
        };
        let registry = fetch_perp_dexs(info).await.unwrap_or_default();
        let (asset_idx, _sz_dec) = match get_asset_meta_for_coin(info, &coin, &registry).await {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry. If using a builder DEX coin (e.g. xyz:CL), run `hyperliquid-plugin dex-list`."));
                return Ok(());
            }
        };
        let action = build_cancel_action(asset_idx, oid);

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "preview": {
                    "mode": "single",
                    "coin": coin,
                    "assetIndex": asset_idx,
                    "orderId": oid,
                    "nonce": nonce
                },
                "action": action
            }))?
        );

        if args.dry_run {
            eprintln!("\n[DRY RUN] Cancel not signed or submitted.");
            return Ok(());
        }
        if !args.confirm {
            eprintln!("\n[PREVIEW] Add --confirm to sign and submit this cancellation.");
            return Ok(());
        }

        let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(&format!("{:#}", e), "SIGNING_FAILED", "Retry the command. If the issue persists, check onchainos status."));
                return Ok(());
            }
        };
        let result = match submit_exchange_request(exchange, signed).await {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(&format!("{:#}", e), "TX_SUBMIT_FAILED", "Retry the command. If the issue persists, check onchainos status."));
                return Ok(());
            }
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "mode": "single",
                "coin": coin,
                "orderId": oid,
                "result": result
            }))?
        );
        return Ok(());
    }

    // Case 2: batch by coin or --all — fetch open orders first.
    // HIP-3: --coin can carry dex prefix (e.g. "xyz:CL") which routes the open-orders
    // query to the right builder DEX; otherwise default DEX. Without --coin, the batch
    // cancels all default-DEX orders only (per-DEX cancel-all not supported in v0.4.0).
    let (cancel_dex_opt, normalized_filter) = match args.coin.as_deref() {
        Some(c) => {
            let (dex, base) = parse_coin(c);
            let filt = if dex.is_some() {
                format!("{}:{}", dex.as_ref().unwrap(), base.to_uppercase())
            } else {
                normalize_coin(&base)
            };
            (dex, Some(filt))
        }
        None => (None, None),
    };
    let open_orders = match get_open_orders_for_dex(info, &wallet, cancel_dex_opt.as_deref()).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
    let empty_vec = vec![];
    let all_orders = open_orders.as_array().unwrap_or(&empty_vec);

    if all_orders.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "message": "No open orders to cancel."
        }))?);
        return Ok(());
    }

    // Filter by coin if provided (already normalized, includes dex prefix when present)
    let coin_filter = normalized_filter;

    let to_cancel: Vec<_> = all_orders
        .iter()
        .filter(|o| {
            if let Some(ref f) = coin_filter {
                // case-insensitive compare on both sides (HIP-3 dex prefix is lowercase
                // while filter may have been built from user's mixed-case input)
                o["coin"].as_str().map(|c| c.eq_ignore_ascii_case(f)).unwrap_or(false)
            } else {
                true // --all
            }
        })
        .collect();

    if to_cancel.is_empty() {
        let msg = if let Some(ref f) = coin_filter {
            format!("No open orders found for {}.", f)
        } else {
            "No open orders to cancel.".to_string()
        };
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "message": msg
        }))?);
        return Ok(());
    }

    // Build asset index map from meta (one call instead of N)
    let meta = match get_meta(info).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };
    let universe = match meta["universe"].as_array() {
        Some(v) => v,
        None => {
            println!("{}", super::error_response("meta.universe missing", "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };

    let get_asset_idx = |coin_name: &str| -> Option<usize> {
        let upper = coin_name.to_uppercase();
        universe
            .iter()
            .enumerate()
            .find(|(_, a)| a["name"].as_str().map(|n| n.to_uppercase()) == Some(upper.clone()))
            .map(|(i, _)| i)
    };

    let mut batch: Vec<(usize, u64)> = Vec::new();
    let mut preview_list = Vec::new();

    for o in &to_cancel {
        let coin_name = o["coin"].as_str().unwrap_or("?");
        let oid = match o["oid"].as_u64() {
            Some(id) => id,
            None => continue,
        };
        let limit_px = o["limitPx"].as_str().unwrap_or("?");
        let sz = o["sz"].as_str().unwrap_or("?");

        let asset_idx = match get_asset_idx(coin_name) {
            Some(i) => i,
            None => {
                println!("{}", super::error_response(&format!("Coin '{}' not found in universe", coin_name), "INVALID_ARGUMENT", "Check the coin symbol and retry."));
                return Ok(());
            }
        };

        batch.push((asset_idx, oid));
        preview_list.push(serde_json::json!({
            "coin": coin_name,
            "oid": oid,
            "limitPrice": limit_px,
            "size": sz
        }));
    }

    let action = build_batch_cancel_action(&batch);

    let mode = if coin_filter.is_some() {
        format!("cancel-by-coin ({})", coin_filter.as_deref().unwrap_or("?"))
    } else {
        "cancel-all".to_string()
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "mode": mode,
                "count": batch.len(),
                "orders": preview_list,
                "nonce": nonce
            },
            "action": action
        }))?
    );

    if args.dry_run {
        eprintln!("\n[DRY RUN] Cancel not signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        eprintln!("\n[PREVIEW] Add --confirm to sign and submit this batch cancellation.");
        return Ok(());
    }

    let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "SIGNING_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "TX_SUBMIT_FAILED", "Retry the command. If the issue persists, check onchainos status."));
            return Ok(());
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "cancelled": batch.len(),
            "result": result
        }))?
    );

    Ok(())
}
