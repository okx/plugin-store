use anyhow::Result;
use clap::Args;

use crate::api::{fetch_pools, parse_pool};

#[derive(Args, Debug)]
pub struct ReservesArgs {
    /// Filter by asset symbol (e.g. SUI, USDC, USDT)
    #[arg(long)]
    pub asset: Option<String>,

    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: ReservesArgs) -> Result<()> {
    let pools_raw = fetch_pools().await?;

    let mut pools: Vec<_> = pools_raw
        .iter()
        .filter_map(|v| parse_pool(v))
        .collect();

    // Sort by total supply (descending)
    pools.sort_by(|a, b| {
        b.total_supply_tokens()
            .partial_cmp(&a.total_supply_tokens())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Filter by asset if requested
    if let Some(ref sym) = args.asset {
        let sym_up = sym.to_uppercase();
        pools.retain(|p| p.symbol.to_uppercase() == sym_up);
        if pools.is_empty() {
            anyhow::bail!("No pool found for asset '{}'", sym);
        }
    }

    if args.json {
        let out: Vec<_> = pools
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id,
                    "symbol": p.symbol,
                    "coin_type": p.coin_type,
                    "pool_id": p.pool_id,
                    "oracle_price_usd": format!("{:.4}", p.oracle_price),
                    "supply_apy_pct": format!("{:.4}", p.supply_apy_pct()),
                    "borrow_apy_pct": format!("{:.4}", p.borrow_apy_pct()),
                    "supply_incentive_apy_pct": format!("{:.4}", p.supply_incentive_apy),
                    "borrow_incentive_apy_pct": format!("{:.4}", p.borrow_incentive_apy),
                    "total_supply_tokens": format!("{:.4}", p.total_supply_tokens()),
                    "total_borrow_tokens": format!("{:.4}", p.total_borrow_tokens()),
                    "utilization_pct": format!("{:.2}", p.utilization_pct()),
                    "max_ltv_pct": format!("{:.2}", p.ltv_pct()),
                    "is_isolated": p.is_isolated,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Human-readable table
    println!(
        "\n{:<10} {:>10} {:>10} {:>10} {:>10} {:>12} {:>12} {:>10}",
        "Asset", "Price", "Supply%", "Borrow%", "Util%", "TotalSupply", "TotalBorrow", "MaxLTV%"
    );
    println!("{}", "-".repeat(90));

    for p in &pools {
        println!(
            "{:<10} {:>10} {:>10} {:>10} {:>10} {:>12} {:>12} {:>10}",
            p.symbol,
            format!("${:.4}", p.oracle_price),
            format!("{:.2}%", p.supply_apy_pct() + p.supply_incentive_apy),
            format!("{:.2}%", p.borrow_apy_pct()),
            format!("{:.2}%", p.utilization_pct()),
            format_large(p.total_supply_tokens()),
            format_large(p.total_borrow_tokens()),
            format!("{:.1}%", p.ltv_pct()),
        );
    }

    println!("\nNote: Supply% includes base + incentive APY. Rates sourced from NAVI open API.");
    Ok(())
}

fn format_large(v: f64) -> String {
    if v >= 1_000_000.0 {
        format!("{:.2}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.2}K", v / 1_000.0)
    } else {
        format!("{:.4}", v)
    }
}
