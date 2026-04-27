use anyhow::Result;
use reqwest::Client;

use crate::api::{get_open_orders, get_pre_migration_orders, OpenOrder};
use crate::auth::ensure_credentials;
use crate::config::OrderVersion;
use crate::onchainos::get_wallet_address;

/// List open orders for the authenticated user.
///
/// `state`: "OPEN", "MATCHED", "DELAYED", or "UNMATCHED".
/// `only_v1`: when true, show only V1-signed orders placed before the CLOB v2 upgrade.
///            Also queries `/data/pre-migration-orders` and merges results so no V1
///            order is missed during the migration window.
pub async fn run(state: &str, only_v1: bool, limit: Option<usize>) -> Result<()> {
    let client = Client::new();
    let signer_addr = get_wallet_address().await?;
    let creds = ensure_credentials(&client, &signer_addr).await?;

    // For --v1, also query the pre-migration endpoint and deduplicate by order_id.
    // This ensures orders placed on the V1 exchange before the cutover are not missed
    // if Polymarket routes them to a separate backing store during the transition window.
    let orders: Vec<OpenOrder> = if only_v1 {
        let (live, pre_migration) = tokio::join!(
            get_open_orders(&client, &signer_addr, &creds, state),
            get_pre_migration_orders(&client, &signer_addr, &creds),
        );
        let mut merged = live.unwrap_or_default();
        let existing_ids: std::collections::HashSet<String> =
            merged.iter().map(|o| o.order_id.clone()).collect();
        for o in pre_migration.unwrap_or_default() {
            if !existing_ids.contains(&o.order_id) {
                merged.push(o);
            }
        }
        merged
    } else {
        get_open_orders(&client, &signer_addr, &creds, state).await?
    };

    let filtered: Vec<serde_json::Value> = orders
        .iter()
        .filter(|o| !only_v1 || o.is_v1())
        .take(limit.unwrap_or(usize::MAX))
        .map(|o| {
            let version_str = match o.version() {
                OrderVersion::V1 => "v1",
                OrderVersion::V2 => "v2",
            };
            serde_json::json!({
                "order_id": o.order_id,
                "order_version": version_str,
                "status": o.status,
                "condition_id": o.condition_id,
                "token_id": o.token_id,
                "side": o.side,
                "price": o.price,
                "original_size": o.original_size,
                "size_matched": o.size_matched,
                "created_at": o.created_at,
            })
        })
        .collect();

    let v1_count = orders.iter().filter(|o| o.is_v1()).count();
    let v2_count = orders.iter().filter(|o| !o.is_v1()).count();

    // In POLY_PROXY mode, the CLOB indexes orders by maker (proxy wallet), and the
    // authenticated /orders endpoint returns orders for the signing address (EOA).
    // Proxy wallet orders may not appear here — verify via the public order book or web UI.
    use crate::config::TradingMode;
    let poly_proxy_note = match &creds.mode {
        TradingMode::PolyProxy => Some(format!(
            "POLY_PROXY mode: orders are placed with the proxy wallet ({}) as maker. \
             The CLOB /orders endpoint returns orders for the EOA signer — proxy wallet orders \
             may not appear here. Check https://polymarket.com for the full order list.",
            creds.proxy_wallet.as_deref().unwrap_or("unknown")
        )),
        _ => None,
    };

    let mut out = serde_json::json!({
        "ok": true,
        "data": {
            "orders": filtered,
            "total": filtered.len(),
            "state": state,
        }
    });

    if let Some(note) = poly_proxy_note {
        out["data"]["poly_proxy_note"] = serde_json::json!(note);
    }

    // Surface a migration notice if V1 orders are present — these remain fillable
    // during the V1 deprecation window but will stop matching after V1 sunset.
    if v1_count > 0 && !only_v1 {
        out["data"]["migration_notice"] = serde_json::json!(format!(
            "{} V1 order(s) detected (placed before the CLOB v2 upgrade on 2026-04-21). \
             These remain fillable during the V1 migration window. \
             Use `polymarket orders --v1` to filter them. \
             Run `polymarket cancel --all` if you want to clear them before V1 sunset.",
            v1_count
        ));
        out["data"]["v1_count"] = serde_json::json!(v1_count);
        out["data"]["v2_count"] = serde_json::json!(v2_count);
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
