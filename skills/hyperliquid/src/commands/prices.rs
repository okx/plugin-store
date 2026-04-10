use clap::Args;
use crate::api::get_all_mids;
use crate::config::{info_url, normalize_coin};

#[derive(Args)]
pub struct PricesArgs {
    /// Specific coin to get price for (e.g. BTC, ETH, SOL).
    /// If omitted, returns all market mid prices.
    #[arg(long)]
    pub coin: Option<String>,
}

pub async fn run(args: PricesArgs) -> anyhow::Result<()> {
    let url = info_url();
    let mids = get_all_mids(url).await?;

    match args.coin {
        Some(coin) => {
            let coin_upper = normalize_coin(&coin);
            match mids.get(&coin_upper) {
                Some(price) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "ok": true,
                            "coin": coin_upper,
                            "midPrice": price
                        }))?
                    );
                }
                None => {
                    anyhow::bail!(
                        "Coin '{}' not found. Check spelling or run `hyperliquid prices` without --coin to list all coins.",
                        coin_upper
                    );
                }
            }
        }
        None => {
            // Return all prices sorted alphabetically
            let obj = mids
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Unexpected allMids response format"))?;

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
                    "count": prices_map.len(),
                    "prices": prices_map
                }))?
            );
        }
    }

    Ok(())
}
