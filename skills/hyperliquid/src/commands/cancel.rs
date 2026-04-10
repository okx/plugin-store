use clap::Args;
use crate::api::{get_asset_index, get_meta, get_open_orders};
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

    let wallet = resolve_wallet(CHAIN_ID)?;

    // ── Determine which orders to cancel ──────────────────────────────────────

    // Case 1: single order by ID
    if let Some(oid) = args.order_id {
        let coin = args
            .coin
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--coin is required when using --order-id"))?;
        let coin = normalize_coin(coin);
        let asset_idx = get_asset_index(info, &coin).await?;
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
            println!("\n[DRY RUN] Cancel not signed or submitted.");
            return Ok(());
        }
        if !args.confirm {
            println!("\n[PREVIEW] Add --confirm to sign and submit this cancellation.");
            return Ok(());
        }

        let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
        let result = submit_exchange_request(exchange, signed).await?;
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

    // Case 2: batch by coin or --all — fetch open orders first
    let open_orders = get_open_orders(info, &wallet).await?;
    let empty_vec = vec![];
    let all_orders = open_orders.as_array().unwrap_or(&empty_vec);

    if all_orders.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "message": "No open orders to cancel."
        }))?);
        return Ok(());
    }

    // Filter by coin if provided
    let coin_filter = args.coin.as_deref().map(normalize_coin);

    let to_cancel: Vec<_> = all_orders
        .iter()
        .filter(|o| {
            if let Some(ref f) = coin_filter {
                o["coin"].as_str().map(|c| c.to_uppercase()) == Some(f.clone())
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
    let meta = get_meta(info).await?;
    let universe = meta["universe"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("meta.universe missing"))?;

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

        let asset_idx = get_asset_idx(coin_name)
            .ok_or_else(|| anyhow::anyhow!("Coin '{}' not found in universe", coin_name))?;

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
        println!("\n[DRY RUN] Cancel not signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        println!("\n[PREVIEW] Add --confirm to sign and submit this batch cancellation.");
        return Ok(());
    }

    let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

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
