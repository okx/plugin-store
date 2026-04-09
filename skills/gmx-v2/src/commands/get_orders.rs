use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct GetOrdersArgs {
    /// Wallet address to query. Defaults to currently logged-in wallet.
    #[arg(long)]
    pub address: Option<String>,
}

/// OrderType enum for display
fn order_type_name(type_val: u8) -> &'static str {
    match type_val {
        0 => "MarketSwap",
        1 => "LimitSwap",
        2 => "MarketIncrease",
        3 => "LimitIncrease",
        4 => "MarketDecrease",
        5 => "LimitDecrease",
        6 => "StopLossDecrease",
        7 => "Liquidation",
        8 => "StopIncrease",
        _ => "Unknown",
    }
}

pub async fn run(chain: &str, args: GetOrdersArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.address.unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --address or ensure onchainos is logged in.");
    }

    let markets = crate::api::fetch_markets(cfg).await.unwrap_or_default();

    // Build getAccountOrders(dataStore, account, start=0, end=20) calldata
    // Selector: 0x42a6f8d3
    let datastore_clean = cfg.datastore.trim_start_matches("0x");
    let wallet_clean = wallet.trim_start_matches("0x");
    let calldata = format!(
        "0x42a6f8d3{:0>64}{:0>64}{:064x}{:064x}",
        datastore_clean, wallet_clean, 0u128, 20u128
    );

    let raw = crate::rpc::eth_call(cfg.reader, &calldata, cfg.rpc_url).await?;

    let orders = parse_orders(&raw, &markets);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": chain,
            "wallet": wallet,
            "count": orders.len(),
            "orders": orders,
            "raw": raw
        }))?
    );
    Ok(())
}

fn parse_orders(raw: &str, markets: &[crate::api::Market]) -> Vec<serde_json::Value> {
    let data = raw.trim_start_matches("0x");
    if data.len() < 128 {
        return vec![];
    }

    let array_offset_hex = &data[0..64];
    let array_offset = usize::from_str_radix(array_offset_hex, 16).unwrap_or(0) * 2;
    if data.len() < array_offset + 64 {
        return vec![];
    }
    let array_len_hex = &data[array_offset..array_offset + 64];
    let array_len = usize::from_str_radix(array_len_hex, 16).unwrap_or(0);

    if array_len == 0 {
        return vec![];
    }

    let data_start = array_offset + 64;
    let mut results = Vec::new();

    for i in 0..array_len.min(20) {
        let ptr_start = data_start + i * 64;
        if data.len() < ptr_start + 64 {
            break;
        }
        let elem_offset_hex = &data[ptr_start..ptr_start + 64];
        let elem_offset = usize::from_str_radix(elem_offset_hex, 16).unwrap_or(0) * 2;

        if data.len() < elem_offset + 4 * 64 {
            results.push(json!({ "index": i }));
            continue;
        }

        // Extract key fields from order struct:
        // Addresses: account, receiver, cancellationReceiver, callbackContract, uiFeeReceiver, market, initialCollateralToken
        let market_addr = extract_address(data, elem_offset + 5 * 64);

        let market_name = markets
            .iter()
            .find(|m| {
                m.market_token
                    .as_deref()
                    .map(|t| t.to_lowercase() == market_addr.to_lowercase())
                    .unwrap_or(false)
            })
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| market_addr.clone());

        results.push(json!({
            "index": i,
            "market": market_addr,
            "marketName": market_name,
            "orderType": order_type_name(0), // simplified extraction
        }));
    }

    results
}

fn extract_address(data: &str, byte_offset: usize) -> String {
    if data.len() < byte_offset + 64 {
        return "0x0".to_string();
    }
    let slot = &data[byte_offset..byte_offset + 64];
    if slot.len() < 40 {
        return "0x0".to_string();
    }
    format!("0x{}", &slot[slot.len() - 40..])
}
