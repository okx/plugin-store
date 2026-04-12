use clap::Args;
use serde_json::json;

#[derive(Args)]
pub struct ListMarketsArgs {
    /// Show only trading markets (skip swap-only markets with no indexToken)
    #[arg(long, default_value_t = true)]
    pub trading_only: bool,
}

pub async fn run(chain: &str, args: ListMarketsArgs) -> anyhow::Result<()> {
    let cfg = crate::config::get_chain_config(chain)?;
    let markets = crate::api::fetch_markets(cfg).await?;

    let filtered: Vec<_> = markets
        .iter()
        .filter(|m| {
            if args.trading_only {
                // Skip swap-only markets (indexToken is null/empty)
                m.index_token
                    .as_deref()
                    .map(|t| !t.is_empty() && t != "0x0000000000000000000000000000000000000000")
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .map(|m| {
            let liq_long = m
                .available_liquidity_long
                .as_deref()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let liq_short = m
                .available_liquidity_short
                .as_deref()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let oi_long = m
                .open_interest_long
                .as_deref()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let oi_short = m
                .open_interest_short
                .as_deref()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);

            // GMX stats API returns rates as (annual_rate_decimal × FLOAT_PRECISION)
            // where FLOAT_PRECISION = 10^30. So: annual_pct = raw / 10^30 * 100 = raw / 10^28
            let to_annual = |raw: Option<&str>| -> String {
                raw.and_then(|s| s.parse::<f64>().ok())
                    .map(|r| format!("{:.4}%", r / 1e28))
                    .unwrap_or_else(|| "0.0000%".to_string())
            };
            json!({
                "name": m.name,
                "marketToken": m.market_token,
                "indexToken": m.index_token,
                "longToken": m.long_token,
                "shortToken": m.short_token,
                "availableLiquidityLong_usd": format!("{:.2}", liq_long as f64 / 1e30),
                "availableLiquidityShort_usd": format!("{:.2}", liq_short as f64 / 1e30),
                "openInterestLong_usd": format!("{:.2}", oi_long as f64 / 1e30),
                "openInterestShort_usd": format!("{:.2}", oi_short as f64 / 1e30),
                "fundingRateLong_annual": to_annual(m.funding_rate_long.as_deref()),
                "fundingRateShort_annual": to_annual(m.funding_rate_short.as_deref()),
                "borrowingRateLong_annual": to_annual(m.borrowing_rate_long.as_deref()),
                "borrowingRateShort_annual": to_annual(m.borrowing_rate_short.as_deref()),
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "chain": chain,
            "count": filtered.len(),
            "markets": filtered
        }))?
    );
    Ok(())
}
