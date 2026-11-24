use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct GetPositionsArgs {
    /// Wallet address to query. Defaults to currently logged-in wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(chain: &str, args: GetPositionsArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;

    let wallet = args.address.unwrap_or_else(|| {
        crate::onchainos::resolve_wallet(cfg.chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot determine wallet address. Pass --address or ensure onchainos is logged in.");
    }

    // Fetch current prices for PnL calculation
    let tickers = crate::api::fetch_prices(cfg).await.unwrap_or_default();
    // Fetch markets for name resolution
    let markets = crate::api::fetch_markets(cfg).await.unwrap_or_default();
    // Fetch token decimals for price display
    let token_infos = crate::api::fetch_tokens(cfg).await.unwrap_or_default();

    // Build getAccountPositions(dataStore, account, start=0, end=20) calldata
    // Selector: 0x77cfb162
    let datastore_clean = cfg.datastore.trim_start_matches("0x");
    let wallet_clean = wallet.trim_start_matches("0x");
    let calldata = format!(
        "0x77cfb162{:0>64}{:0>64}{:064x}{:064x}",
        datastore_clean, wallet_clean, 0u128, 20u128
    );

    let raw = crate::rpc::eth_call(cfg.reader, &calldata, cfg.rpc_url).await?;

    let positions = parse_positions(&raw, &tickers, &markets, &token_infos);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": chain,
            "wallet": wallet,
            "count": positions.len(),
            "positions": positions,
        }))?
    );
    Ok(())
}

fn parse_positions(
    raw: &str,
    tickers: &[crate::api::PriceTicker],
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

    // Position.Props is a static 14-word struct:
    //   word  0: account
    //   word  1: market
    //   word  2: collateralToken
    //   word  3: sizeInUsd          (10^30 precision)
    //   word  4: sizeInTokens       (index token units)
    //   word  5: collateralAmount   (collateral token units)
    //   words 6-10: funding/borrowing per-size fields
    //   word 11: increasedAtTime    (unix timestamp)
    //   word 12: decreasedAtTime    (unix timestamp, 0 if never)
    //   word 13: isLong             (bool)
    const WORDS_PER_POSITION: usize = 14;
    const HEX_CHARS_PER_WORD: usize = 64;

    let mut results = Vec::new();
    let data_start = array_offset + HEX_CHARS_PER_WORD;

    for i in 0..array_len.min(20) {
        let elem_base = data_start + i * WORDS_PER_POSITION * HEX_CHARS_PER_WORD;
        if data.len() < elem_base + 14 * HEX_CHARS_PER_WORD {
            results.push(json!({ "index": i, "error": "truncated data" }));
            continue;
        }

        let account_addr  = extract_address(data, elem_base);
        let market_addr   = extract_address(data, elem_base + 1 * HEX_CHARS_PER_WORD);
        let collateral_addr = extract_address(data, elem_base + 2 * HEX_CHARS_PER_WORD);

        let size_in_usd_raw    = extract_u128(data, elem_base + 3 * HEX_CHARS_PER_WORD);
        let size_in_tokens_raw = extract_u128(data, elem_base + 4 * HEX_CHARS_PER_WORD);
        let collateral_raw     = extract_u128(data, elem_base + 5 * HEX_CHARS_PER_WORD);
        let is_long            = extract_u128(data, elem_base + 13 * HEX_CHARS_PER_WORD) != 0;

        let size_usd = size_in_usd_raw as f64 / 1e30;

        // Market info
        let market_info = markets.iter().find(|m| {
            m.market_token.as_deref()
                .map(|t| t.to_lowercase() == market_addr.to_lowercase())
                .unwrap_or(false)
        });
        let market_name = market_info
            .and_then(|m| m.name.clone())
            .unwrap_or_else(|| market_addr.clone());
        let index_token = market_info.and_then(|m| m.index_token.clone());

        // Index token decimals (for price and entry price calculation)
        let index_decimals = index_token.as_deref()
            .and_then(|addr| token_infos.iter()
                .find(|ti| ti.address.as_deref().map(|a| a.to_lowercase()) == Some(addr.to_lowercase()))
                .and_then(|ti| ti.decimals))
            .unwrap_or(18u8);

        // Collateral token decimals
        let collateral_decimals = token_infos.iter()
            .find(|ti| ti.address.as_deref().map(|a| a.to_lowercase()) == Some(collateral_addr.to_lowercase()))
            .and_then(|ti| ti.decimals)
            .unwrap_or(18u8);

        let collateral_display = collateral_raw as f64 / 10f64.powi(collateral_decimals as i32);
        let leverage = if collateral_display > 0.0 { size_usd / collateral_display } else { 0.0 };

        // Current price
        let current_price_usd = index_token.as_deref().and_then(|addr| {
            crate::api::find_price(tickers, addr).map(|t| {
                let raw = t.min_price.as_deref().unwrap_or("0").parse::<u128>().unwrap_or(0);
                crate::api::raw_price_to_usd(raw, index_decimals)
            })
        });

        // Entry price = sizeInUsd / sizeInTokens (adjusted for decimals)
        let entry_price_usd = if size_in_tokens_raw > 0 && size_in_usd_raw > 0 {
            // entryPrice = sizeInUsd * 10^indexDecimals / (sizeInTokens * 10^30)
            let factor = 10f64.powi(index_decimals as i32 - 30);
            size_in_usd_raw as f64 * factor / size_in_tokens_raw as f64
        } else {
            0.0
        };

        // Unrealized PnL
        let unrealized_pnl = current_price_usd.map(|curr| {
            if entry_price_usd > 0.0 {
                let price_change = if is_long { curr - entry_price_usd } else { entry_price_usd - curr };
                price_change / entry_price_usd * size_usd
            } else {
                0.0
            }
        });

        results.push(json!({
            "index": i,
            "account": account_addr,
            "market": market_addr,
            "marketName": market_name,
            "collateralToken": collateral_addr,
            "direction": if is_long { "LONG" } else { "SHORT" },
            "sizeUsd": format!("{:.4}", size_usd),
            "collateralUsd": format!("{:.4}", collateral_display),
            "leverage": format!("{:.2}x", leverage),
            "entryPrice_usd": format!("{:.4}", entry_price_usd),
            "currentPrice_usd": current_price_usd.map(|p| format!("{:.4}", p)),
            "unrealizedPnl_usd": unrealized_pnl.map(|p| format!("{:.4}", p)),
        }));
    }

    results
}

fn extract_u128(data: &str, hex_offset: usize) -> u128 {
    if data.len() < hex_offset + 64 {
        return 0;
    }
    let slot = &data[hex_offset..hex_offset + 64];
    // Lower 128 bits (last 32 hex chars)
    u128::from_str_radix(&slot[32..], 16).unwrap_or(0)
}

fn extract_address(data: &str, hex_offset: usize) -> String {
    if data.len() < hex_offset + 64 {
        return "0x0".to_string();
    }
    let slot = &data[hex_offset..hex_offset + 64];
    format!("0x{}", &slot[slot.len() - 40..])
}
