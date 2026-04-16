use clap::Args;
use crate::api::{get_spot_asset_meta, get_open_orders};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, resolve_wallet};
use crate::signing::{build_cancel_action, build_batch_cancel_action, submit_exchange_request};

#[derive(Args)]
pub struct SpotCancelArgs {
    /// Cancel a specific order by ID. Also requires --coin.
    #[arg(long)]
    pub order_id: Option<u64>,

    /// Base token symbol (e.g. PURR, HYPE).
    /// With --order-id: required to resolve asset index.
    /// Without --order-id: cancels ALL open spot orders for this token.
    #[arg(long)]
    pub coin: Option<String>,

    /// Cancel ALL open spot orders across all tokens.
    #[arg(long, conflicts_with_all = ["order_id", "coin"])]
    pub all: bool,

    /// Dry run — preview payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: SpotCancelArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();
    let nonce = now_ms();

    let wallet = resolve_wallet(CHAIN_ID)?;

    // ── Case 1: single order by ID ────────────────────────────────────────────
    if let Some(oid) = args.order_id {
        let coin_str = args
            .coin
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--coin is required when using --order-id"))?;
        let coin = normalize_coin(coin_str);

        let (asset_idx, market_idx, _) = get_spot_asset_meta(info, &coin).await?;
        let action = build_cancel_action(asset_idx, oid);

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "preview": {
                    "market": "spot",
                    "mode": "single",
                    "coin": coin,
                    "marketIndex": market_idx,
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

        let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
        let result = submit_exchange_request(exchange, signed).await?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "market": "spot",
                "coin": coin,
                "orderId": oid,
                "result": result
            }))?
        );
        return Ok(());
    }

    // ── Case 2/3: batch cancel by coin or --all ───────────────────────────────
    // Spot orders in openOrders have coin = "@{market_idx}" (the market name).
    // Resolve the expected coin filter before fetching orders.

    let spot_coin_filter: Option<(String, usize, usize)> = if let Some(ref c) = args.coin {
        let coin = normalize_coin(c);
        let (asset_idx, market_idx, _) = get_spot_asset_meta(info, &coin).await?;
        Some((coin, asset_idx, market_idx))
    } else {
        None
    };

    let open_orders = get_open_orders(info, &wallet).await?;
    let empty_vec = vec![];
    let all_orders = open_orders.as_array().unwrap_or(&empty_vec);

    if all_orders.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "message": "No open orders to cancel."
            }))?
        );
        return Ok(());
    }

    // Spot orders have coin starting with "@" (market name) while perp orders use coin symbol
    let to_cancel: Vec<_> = all_orders
        .iter()
        .filter(|o| {
            let coin_field = o["coin"].as_str().unwrap_or("");
            match &spot_coin_filter {
                Some((_, _, mkt_idx)) => {
                    // Match by exact market name "@{mkt_idx}"
                    coin_field == format!("@{}", mkt_idx)
                }
                None => {
                    // --all: match any spot order (coin starts with "@")
                    coin_field.starts_with('@')
                }
            }
        })
        .collect();

    if to_cancel.is_empty() {
        let msg = match &spot_coin_filter {
            Some((coin, _, _)) => format!("No open spot orders found for {}.", coin),
            None => "No open spot orders to cancel.".to_string(),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "message": msg
            }))?
        );
        return Ok(());
    }

    // Build batch cancel — for spot orders the asset_idx = 10000 + market_idx.
    // The market_idx can be parsed from the coin field "@N".
    let mut batch: Vec<(usize, u64)> = Vec::new();
    let mut preview_list = Vec::new();

    for o in &to_cancel {
        let coin_field = o["coin"].as_str().unwrap_or("?");
        let oid = match o["oid"].as_u64() {
            Some(id) => id,
            None => continue,
        };
        let limit_px = o["limitPx"].as_str().unwrap_or("?");
        let sz = o["sz"].as_str().unwrap_or("?");

        // Parse market_idx from "@N" — or use pre-resolved value if we have it
        let asset_idx = match &spot_coin_filter {
            Some((_, ai, _)) => *ai,
            None => {
                // coin_field is "@N"
                let n: usize = coin_field
                    .trim_start_matches('@')
                    .parse()
                    .unwrap_or(0);
                10000 + n
            }
        };

        batch.push((asset_idx, oid));
        preview_list.push(serde_json::json!({
            "coin": coin_field,
            "assetIndex": asset_idx,
            "oid": oid,
            "limitPrice": limit_px,
            "size": sz
        }));
    }

    let action = build_batch_cancel_action(&batch);

    let mode = match &spot_coin_filter {
        Some((coin, _, _)) => format!("cancel-by-coin ({})", coin),
        None => "cancel-all-spot".to_string(),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": {
                "market": "spot",
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

    let signed = onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false)?;
    let result = submit_exchange_request(exchange, signed).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "market": "spot",
            "cancelled": batch.len(),
            "result": result
        }))?
    );

    Ok(())
}
