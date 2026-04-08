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

    // Build getAccountPositions(dataStore, account, start=0, end=20) calldata
    // Selector: 0x77cfb162
    let datastore_clean = cfg.datastore.trim_start_matches("0x");
    let wallet_clean = wallet.trim_start_matches("0x");
    let calldata = format!(
        "0x77cfb162{:0>64}{:0>64}{:064x}{:064x}",
        datastore_clean, wallet_clean, 0u128, 20u128
    );

    let raw = crate::rpc::eth_call(cfg.reader, &calldata, cfg.rpc_url).await?;

    // Parse the response — positions are ABI-encoded structs
    // The raw response is a complex tuple; we parse key fields by position
    // For display we show what we can extract, and include the raw hex for completeness
    let positions = parse_positions(&raw, &tickers, &markets);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": chain,
            "wallet": wallet,
            "count": positions.len(),
            "positions": positions,
            "raw": raw
        }))?
    );
    Ok(())
}

fn parse_positions(
    raw: &str,
    tickers: &[crate::api::PriceTicker],
    markets: &[crate::api::Market],
) -> Vec<serde_json::Value> {
    // ABI-encoded array of Position structs
    // The raw data starts with an offset, then array length, then elements
    // Each position is a tuple with many fields; we extract key ones

    let data = raw.trim_start_matches("0x");
    if data.len() < 128 {
        return vec![];
    }

    // Slot 0: offset to array start
    // Slot 1 (at offset): array length
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

    // Each position struct is a fixed-size tuple starting at known offsets
    // Position struct fields (simplified extraction):
    // Slot relative positions in each struct vary due to dynamic content
    // We use a simplified heuristic approach — extract what we can
    let mut results = Vec::new();

    // Look at each 32-byte slot after the array header for addresses
    let data_start = array_offset + 64; // after length word

    // Each position has an offset pointer in the head
    for i in 0..array_len.min(20) {
        let ptr_start = data_start + i * 64;
        if data.len() < ptr_start + 64 {
            break;
        }
        let elem_offset_hex = &data[ptr_start..ptr_start + 64];
        let elem_offset = usize::from_str_radix(elem_offset_hex, 16).unwrap_or(0) * 2;

        // Try to extract key fields from this position struct
        // Field layout (approximate — varies by GMX version):
        // +0: account (address)
        // +1: market (address)
        // +2: collateralToken (address)
        // Then Numbers struct, then Flags struct
        let elem_base = elem_offset; // relative to start of full data
        if data.len() < elem_base + 6 * 64 {
            results.push(json!({ "index": i, "raw_offset": elem_offset / 2 }));
            continue;
        }

        let account_addr = extract_address(data, elem_base);
        let market_addr = extract_address(data, elem_base + 64);
        let collateral_addr = extract_address(data, elem_base + 128);

        // Find market name
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

        // Try to get current price for index token of this market
        let index_token = markets
            .iter()
            .find(|m| {
                m.market_token
                    .as_deref()
                    .map(|t| t.to_lowercase() == market_addr.to_lowercase())
                    .unwrap_or(false)
            })
            .and_then(|m| m.index_token.clone());

        let current_price_usd = index_token.as_deref().and_then(|addr| {
            crate::api::find_price(tickers, addr).map(|t| {
                t.min_price
                    .as_deref()
                    .unwrap_or("0")
                    .parse::<u128>()
                    .unwrap_or(0) as f64
                    / 1e30
            })
        });

        results.push(json!({
            "index": i,
            "account": account_addr,
            "market": market_addr,
            "marketName": market_name,
            "collateralToken": collateral_addr,
            "currentPrice_usd": current_price_usd,
        }));
    }

    results
}

fn extract_address(data: &str, byte_offset: usize) -> String {
    let hex_offset = byte_offset; // already in hex chars
    if data.len() < hex_offset + 64 {
        return "0x0".to_string();
    }
    let slot = &data[hex_offset..hex_offset + 64];
    if slot.len() < 40 {
        return "0x0".to_string();
    }
    format!("0x{}", &slot[slot.len() - 40..])
}
