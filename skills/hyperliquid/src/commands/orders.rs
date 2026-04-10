use clap::Args;
use crate::api::get_open_orders;
use crate::config::{info_url, normalize_coin, CHAIN_ID};
use crate::onchainos::resolve_wallet;

#[derive(Args)]
pub struct OrdersArgs {
    /// Filter by coin (e.g. BTC, ETH). If omitted, shows all open orders.
    #[arg(long)]
    pub coin: Option<String>,

    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: OrdersArgs) -> anyhow::Result<()> {
    let url = info_url();

    let address = match args.address {
        Some(addr) => addr,
        None => resolve_wallet(CHAIN_ID)?,
    };

    let orders = get_open_orders(url, &address).await?;

    let empty_vec = vec![];
    let all_orders = orders.as_array().unwrap_or(&empty_vec);

    let coin_filter = args.coin.map(|c| normalize_coin(&c));

    let mut out = Vec::new();
    for o in all_orders {
        let coin = o["coin"].as_str().unwrap_or("?");
        if let Some(ref filter) = coin_filter {
            if coin.to_uppercase() != *filter {
                continue;
            }
        }

        let side_raw = o["side"].as_str().unwrap_or("?");
        let side = match side_raw {
            "B" => "buy",
            "A" => "sell",
            other => other,
        };

        let limit_px = o["limitPx"].as_str().unwrap_or("?");
        let sz = o["sz"].as_str().unwrap_or("?");
        let orig_sz = o["origSz"].as_str().unwrap_or(sz);
        let oid = o["oid"].as_u64().unwrap_or(0);
        let timestamp = o["timestamp"].as_u64().unwrap_or(0);
        let reduce_only = o["reduceOnly"].as_bool().unwrap_or(false);

        // Determine order type label
        let order_type = if reduce_only { "reduce-only (TP/SL)" } else { "limit" };

        out.push(serde_json::json!({
            "oid": oid,
            "coin": coin,
            "side": side,
            "limitPrice": limit_px,
            "size": sz,
            "origSize": orig_sz,
            "type": order_type,
            "timestamp": timestamp
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "address": address,
            "count": out.len(),
            "orders": out
        }))?
    );

    Ok(())
}
