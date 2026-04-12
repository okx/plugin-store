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
    let token_infos = crate::api::fetch_tokens(cfg).await.unwrap_or_default();

    // Build getAccountOrders(dataStore, account, start=0, end=20) calldata
    // Selector: 0x42a6f8d3
    let datastore_clean = cfg.datastore.trim_start_matches("0x");
    let wallet_clean = wallet.trim_start_matches("0x");
    let calldata = format!(
        "0x42a6f8d3{:0>64}{:0>64}{:064x}{:064x}",
        datastore_clean, wallet_clean, 0u128, 20u128
    );

    let raw = crate::rpc::eth_call(cfg.reader, &calldata, cfg.rpc_url).await?;

    let orders = parse_orders(&raw, &markets, &token_infos);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": chain,
            "wallet": wallet,
            "count": orders.len(),
            "orders": orders
        }))?
    );
    Ok(())
}

/// Parse orders from raw ABI bytes.
///
/// Order.Props ABI layout (from on-chain dump analysis):
///
/// Each element = (bytes32 key, Order.Props props) where Order.Props is dynamic.
///
/// Props head (18 words = 576 bytes):
///   word  0: Addresses offset (= 576)
///   word  1: orderType         (uint8)
///   word  2: decreasePositionSwapType (uint8)
///   word  3: sizeDeltaUsd      (uint256, ×10^30)
///   word  4: initialCollateralDeltaAmount (uint256, token units)
///   word  5: triggerPrice      (uint256, price × 10^(30-indexDecimals))
///   word  6: acceptablePrice   (uint256, same format)
///   word  7: executionFee      (uint256, wei)
///   word  8: callbackGasLimit  (uint256)
///   word  9: minOutputAmount   (uint256)
///   word 10: validFromTime     (uint256, unix ts)
///   word 11: isLong            (bool)
///   word 12: shouldUnwrapNativeToken (bool)
///   word 13: autoCancel        (bool)
///   word 14: referralCode      (bytes32)
///   word 15: (reserved)
///   word 16: (reserved)
///   word 17: dataList offset   (dynamic)
///
/// Addresses (at props_base + 576, 9 words):
///   word  0: account
///   word  1: receiver
///   word  2: cancellationReceiver
///   word  3: callbackContract
///   word  4: uiFeeReceiver
///   word  5: market
///   word  6: initialCollateralToken
///   word  7: swapPath offset (relative to Addresses start)
///   word  8: swapPath length (= 0 for most orders)
fn parse_orders(
    raw: &str,
    markets: &[crate::api::Market],
    token_infos: &[crate::api::TokenInfo],
) -> Vec<serde_json::Value> {
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

    // Order is dynamic (Addresses has swapPath[]), so each element has an offset pointer.
    let data_start = array_offset + 64; // right after length word, in hex chars
    let mut results = Vec::new();

    for i in 0..array_len.min(20) {
        let ptr_start = data_start + i * 64;
        if data.len() < ptr_start + 64 {
            break;
        }
        let elem_offset_hex = &data[ptr_start..ptr_start + 64];
        let elem_offset_bytes = usize::from_str_radix(elem_offset_hex, 16).unwrap_or(0);
        let elem_base = data_start + elem_offset_bytes * 2;

        if data.len() < elem_base + 4 * 64 {
            results.push(json!({ "index": i }));
            continue;
        }

        // word 0: bytes32 order key
        let key_hex = &data[elem_base..elem_base + 64];
        let order_key = format!("0x{}", key_hex);

        // word 1: offset to Order.Props (relative to elem_base)
        let props_rel_hex = &data[elem_base + 64..elem_base + 128];
        let props_rel = usize::from_str_radix(props_rel_hex, 16).unwrap_or(0) * 2;
        let props_base = elem_base + props_rel;

        if data.len() < props_base + 18 * 64 {
            results.push(json!({ "index": i, "orderKey": order_key }));
            continue;
        }

        // Props word 0: Addresses offset (relative to props_base, in bytes)
        let addr_off_hex = &data[props_base..props_base + 64];
        let addr_off = usize::from_str_radix(addr_off_hex, 16).unwrap_or(0) * 2;
        let addr_base = props_base + addr_off;

        // Props word 1: orderType
        let order_type_val = extract_u8(data, props_base + 64);

        // Props word 3: sizeDeltaUsd (×10^30)
        let size_delta_raw = extract_u128(data, props_base + 3 * 64);
        let size_usd = size_delta_raw as f64 / 1e30;

        // Props word 4: initialCollateralDeltaAmount
        let collateral_delta_raw = extract_u128(data, props_base + 4 * 64);

        // Props word 5: triggerPrice (price × 10^(30 - indexDecimals))
        let trigger_price_raw = extract_u128(data, props_base + 5 * 64);

        // Props word 6: acceptablePrice
        let acceptable_price_raw = extract_u128(data, props_base + 6 * 64);

        // Props word 11: isLong (bool)
        let is_long = extract_u128(data, props_base + 11 * 64) != 0;

        // Addresses word 5: market
        let market_addr = extract_address(data, addr_base + 5 * 64);

        // Addresses word 6: initialCollateralToken
        let collateral_token = extract_address(data, addr_base + 6 * 64);

        // Market metadata
        let market_info = markets.iter().find(|m| {
            m.market_token
                .as_deref()
                .map(|t| t.to_lowercase() == market_addr.to_lowercase())
                .unwrap_or(false)
        });
        let market_name = market_info
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| market_addr.clone());

        // Index token decimals for price display
        let index_decimals = market_info
            .and_then(|m| m.index_token.as_deref())
            .and_then(|addr| {
                token_infos.iter()
                    .find(|t| t.address.as_deref().map(|a| a.to_lowercase()) == Some(addr.to_lowercase()))
                    .and_then(|t| t.decimals)
            })
            .unwrap_or(18u8);

        // Collateral token decimals
        let collateral_decimals = token_infos.iter()
            .find(|t| t.address.as_deref().map(|a| a.to_lowercase()) == Some(collateral_token.to_lowercase()))
            .and_then(|t| t.decimals)
            .unwrap_or(6u8);

        let trigger_price_usd = crate::api::raw_price_to_usd(trigger_price_raw, index_decimals);
        let acceptable_price_usd = crate::api::raw_price_to_usd(acceptable_price_raw, index_decimals);
        let collateral_delta_fmt = crate::api::format_token_amount(collateral_delta_raw, collateral_decimals);

        results.push(json!({
            "index": i,
            "orderKey": order_key,
            "market": market_addr,
            "marketName": market_name,
            "collateralToken": collateral_token,
            "orderType": order_type_name(order_type_val),
            "direction": if is_long { "LONG" } else { "SHORT" },
            "sizeUsd": format!("{:.4}", size_usd),
            "collateralDelta": collateral_delta_fmt,
            "triggerPrice_usd": format!("{:.4}", trigger_price_usd),
            "acceptablePrice_usd": format!("{:.4}", acceptable_price_usd),
        }));
    }

    results
}

fn extract_u8(data: &str, hex_offset: usize) -> u8 {
    if data.len() < hex_offset + 64 {
        return 0;
    }
    let slot = &data[hex_offset..hex_offset + 64];
    usize::from_str_radix(slot, 16).unwrap_or(0) as u8
}

fn extract_u128(data: &str, hex_offset: usize) -> u128 {
    if data.len() < hex_offset + 64 {
        return 0;
    }
    // u128 can only hold 32 hex chars; take the lower 32 chars to avoid overflow
    let slot = &data[hex_offset..hex_offset + 64];
    let lower = &slot[32..]; // last 16 bytes = 32 hex chars
    u128::from_str_radix(lower, 16).unwrap_or(0)
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

/// Extract just the order keys (bytes32) from raw ABI-encoded getAccountOrders response.
/// Used by place-order to diff pre/post order sets and find the newly created key.
pub fn extract_order_keys(raw: &str) -> Vec<String> {
    let data = raw.trim_start_matches("0x");
    if data.len() < 128 {
        return vec![];
    }
    let array_offset = usize::from_str_radix(&data[0..64], 16).unwrap_or(0) * 2;
    if data.len() < array_offset + 64 {
        return vec![];
    }
    let array_len = usize::from_str_radix(&data[array_offset..array_offset + 64], 16).unwrap_or(0);
    if array_len == 0 {
        return vec![];
    }
    let data_start = array_offset + 64;
    let mut keys = Vec::new();
    for i in 0..array_len.min(20) {
        let ptr_start = data_start + i * 64;
        if data.len() < ptr_start + 64 {
            break;
        }
        let elem_offset_bytes = usize::from_str_radix(&data[ptr_start..ptr_start + 64], 16).unwrap_or(0);
        let elem_base = data_start + elem_offset_bytes * 2;
        if data.len() < elem_base + 64 {
            continue;
        }
        keys.push(format!("0x{}", &data[elem_base..elem_base + 64]));
    }
    keys
}
