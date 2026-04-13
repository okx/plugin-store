use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct GetPricesArgs {
    /// Filter by token symbol (e.g. ETH, BTC). If empty, returns all.
    #[arg(long)]
    pub symbol: Option<String>,
}

pub async fn run(chain: &str, args: GetPricesArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;
    let tickers = crate::api::fetch_prices(cfg).await?;

    // Fetch token decimals for proper price conversion
    let token_infos = crate::api::fetch_tokens(cfg).await.unwrap_or_default();

    // Build address -> decimals map (case-insensitive)
    let decimals_map: std::collections::HashMap<String, u8> = token_infos
        .iter()
        .filter_map(|t| {
            let addr = t.address.as_deref()?.to_lowercase();
            let dec = t.decimals?;
            Some((addr, dec))
        })
        .collect();

    let filtered: Vec<_> = tickers
        .iter()
        .filter(|t| {
            if let Some(sym) = &args.symbol {
                t.token_symbol
                    .as_deref()
                    .map(|s| s.to_lowercase() == sym.to_lowercase())
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .map(|t| {
            let min_raw: u128 = t
                .min_price
                .as_deref()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            let max_raw: u128 = t
                .max_price
                .as_deref()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);

            // Look up token decimals (default 18 for unknown tokens)
            let decimals = t
                .token_address
                .as_deref()
                .and_then(|a| decimals_map.get(&a.to_lowercase()).copied())
                .unwrap_or(18u8);

            let min_usd = crate::api::raw_price_to_usd(min_raw, decimals);
            let max_usd = crate::api::raw_price_to_usd(max_raw, decimals);
            let mid_usd = (min_usd + max_usd) / 2.0;

            json!({
                "tokenAddress": t.token_address,
                "symbol": t.token_symbol,
                "minPrice_usd": format!("{:.4}", min_usd),
                "maxPrice_usd": format!("{:.4}", max_usd),
                "midPrice_usd": format!("{:.4}", mid_usd),
                "minPrice_raw": t.min_price,
                "maxPrice_raw": t.max_price,
                "updatedAt": t.updated_at,
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": chain,
            "count": filtered.len(),
            "prices": filtered
        }))?
    );
    Ok(())
}
