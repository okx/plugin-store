use clap::Args;
use crate::api::{get_open_orders_for_dex, parse_coin};
use crate::config::{info_url, normalize_coin, CHAIN_ID};
use crate::onchainos::resolve_wallet;

#[derive(Args)]
pub struct OrdersArgs {
    /// Filter by coin (e.g. BTC, ETH, or xyz:CL for HIP-3 builder DEX coins).
    /// If omitted, shows all open orders on the selected DEX.
    #[arg(long)]
    pub coin: Option<String>,

    /// HIP-3 builder DEX name (xyz / flx / vntl / hyna / km / cash / para / abcd).
    /// If omitted, queries the default Hyperliquid perp DEX. Each builder DEX has
    /// SEPARATE order books. If --coin contains a DEX prefix (e.g. "xyz:CL"),
    /// the prefix is auto-extracted and overrides this flag.
    #[arg(long)]
    pub dex: Option<String>,

    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: OrdersArgs) -> anyhow::Result<()> {
    let url = info_url();

    let address = match args.address {
        Some(addr) => addr,
        None => match resolve_wallet(CHAIN_ID) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::error_response(&format!("{:#}", e), "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login."));
                return Ok(());
            }
        },
    };

    // Auto-extract DEX from --coin prefix if present
    let (effective_dex, coin_filter) = match &args.coin {
        Some(c) => {
            let (parsed_dex, base) = parse_coin(c);
            let chosen_dex = parsed_dex.or_else(|| args.dex.clone());
            let filter = if let Some(d) = &chosen_dex {
                Some(format!("{}:{}", d, base.to_uppercase()))
            } else {
                Some(normalize_coin(&base))
            };
            (chosen_dex, filter)
        }
        None => (args.dex.clone(), None),
    };
    let dex_arg = effective_dex.as_deref();
    let dex_label = dex_arg.unwrap_or("default").to_string();

    let orders = match get_open_orders_for_dex(url, &address, dex_arg).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };

    let empty_vec = vec![];
    let all_orders = orders.as_array().unwrap_or(&empty_vec);

    let mut out = Vec::new();
    for o in all_orders {
        let coin = o["coin"].as_str().unwrap_or("?");
        if let Some(ref filter) = coin_filter {
            // HL returns builder-DEX coins as "xyz:NVDA" (dex prefix lowercase,
            // symbol uppercase). Filter was built the same way upstream, but
            // earlier code did `coin.to_uppercase() != filter` which uppercases
            // the dex prefix too → never matches. Use case-insensitive compare.
            if !coin.eq_ignore_ascii_case(filter) {
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
            "dex": dex_label,
            "count": out.len(),
            "orders": out
        }))?
    );

    Ok(())
}
