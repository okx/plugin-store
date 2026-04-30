use clap::Args;
use crate::api::{get_all_mids_for_dex, parse_coin};
use crate::config::{info_url, normalize_coin};

#[derive(Args)]
pub struct PricesArgs {
    /// Specific coin to get price for (e.g. BTC, ETH, SOL, or xyz:CL for HIP-3 builder DEX coins).
    /// If omitted, returns all market mid prices for the selected DEX.
    #[arg(long)]
    pub coin: Option<String>,
    /// HIP-3 builder DEX name (xyz / flx / vntl / hyna / km / cash / para / abcd).
    /// If omitted, prices are from the default Hyperliquid perp DEX.
    /// If --coin contains a DEX prefix (e.g. "xyz:CL"), the prefix is auto-extracted
    /// and overrides this flag.
    #[arg(long)]
    pub dex: Option<String>,
}

pub async fn run(args: PricesArgs) -> anyhow::Result<()> {
    let url = info_url();

    // Auto-extract DEX from --coin prefix if present (e.g. "xyz:CL" -> dex=xyz, base=CL)
    let (effective_dex, coin_filter) = match &args.coin {
        Some(c) => {
            let (parsed_dex, base) = parse_coin(c);
            // Coin's prefix beats --dex flag (more specific)
            let chosen_dex = parsed_dex.or_else(|| args.dex.clone());
            // For default-dex coins, lookup uses normalized coin name; for builder dex,
            // the universe stores the FULL prefixed name (e.g. "xyz:CL")
            let lookup_key = if let Some(d) = &chosen_dex {
                format!("{}:{}", d, base.to_uppercase())
            } else {
                normalize_coin(&base)
            };
            (chosen_dex, Some(lookup_key))
        }
        None => (args.dex.clone(), None),
    };

    let dex_arg = effective_dex.as_deref();
    let mids = match get_all_mids_for_dex(url, dex_arg).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry."));
            return Ok(());
        }
    };

    match coin_filter {
        Some(key) => {
            match mids.get(&key) {
                Some(price) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "ok": true,
                            "dex": effective_dex.unwrap_or_else(|| "default".to_string()),
                            "coin": key,
                            "midPrice": price
                        }))?
                    );
                }
                None => {
                    let dex_label = effective_dex.unwrap_or_else(|| "default".to_string());
                    println!("{}", super::error_response(
                        &format!("Coin '{}' not found on {} DEX. Check spelling or run `hyperliquid prices --dex {}` to list all coins on that DEX.", key, dex_label, dex_label),
                        "INVALID_ARGUMENT",
                        "Check the coin symbol or run `hyperliquid dex-list` to see all DEXs and `hyperliquid prices --dex X` to list one."
                    ));
                }
            }
        }
        None => {
            let obj = match mids.as_object() {
                Some(v) => v,
                None => {
                    println!("{}", super::error_response("Unexpected allMids response format", "API_ERROR", "Check your connection and retry."));
                    return Ok(());
                }
            };

            let mut sorted: Vec<(&String, &serde_json::Value)> = obj.iter().collect();
            sorted.sort_by_key(|(k, _)| k.as_str());

            let prices_map: serde_json::Map<String, serde_json::Value> = sorted
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "dex": effective_dex.unwrap_or_else(|| "default".to_string()),
                    "count": prices_map.len(),
                    "prices": prices_map
                }))?
            );
        }
    }

    Ok(())
}
