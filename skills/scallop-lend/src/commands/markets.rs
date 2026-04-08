use anyhow::Result;
use clap::Args;
use reqwest::Client;
use serde_json::Value;

use crate::api::fetch_pool_addresses;
use crate::rpc::get_object;

#[derive(Args, Debug)]
pub struct MarketsArgs {
    /// Filter by asset symbol (e.g. sui, usdc, sca)
    #[arg(long)]
    pub asset: Option<String>,

    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
}

/// Compute utilization rate from cash and debt
fn utilization(cash: u64, debt: u64) -> f64 {
    let total = cash + debt;
    if total == 0 {
        return 0.0;
    }
    debt as f64 / total as f64
}

/// Approximate supply APY based on utilization (simplified linear model)
/// Scallop uses a kink model; here we approximate with a linear estimate.
fn approx_supply_apy(util: f64) -> f64 {
    // Simplified: base 1% + 20% at 80% utilization (kink), then steeper
    let base = 0.01_f64;
    let kink = 0.80_f64;
    let slope1 = 0.20_f64; // 20% APY at kink
    let slope2 = 1.50_f64; // 150% APY at 100%

    let borrow_apy = if util <= kink {
        base + (slope1 / kink) * util
    } else {
        base + slope1 + (slope2 - slope1) * (util - kink) / (1.0 - kink)
    };
    // Supply APY = borrow APY * utilization
    borrow_apy * util
}

/// Approximate borrow APY
fn approx_borrow_apy(util: f64) -> f64 {
    let base = 0.01_f64;
    let kink = 0.80_f64;
    let slope1 = 0.20_f64;
    let slope2 = 1.50_f64;

    if util <= kink {
        base + (slope1 / kink) * util
    } else {
        base + slope1 + (slope2 - slope1) * (util - kink) / (1.0 - kink)
    }
}

/// Parse u64 from a field that may be a string or number in JSON
fn parse_u64(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n.as_u64().unwrap_or(0),
        Value::String(s) => s.parse::<u64>().unwrap_or(0),
        _ => 0,
    }
}

/// Fetch pool object and extract balance sheet data
/// Pool objects are dynamic fields of type:
/// 0x2::dynamic_field::Field<TypeName, reserve::BalanceSheet>
/// where fields.value.cash and fields.value.debt contain the data
async fn fetch_pool_stats(
    client: &Client,
    pool_address: &str,
    _decimals: u8,
) -> Option<(u64, u64)> {
    if let Ok(obj) = get_object(client, pool_address).await {
        let fields = obj.pointer("/data/content/fields")?;

        // Primary path: dynamic field with value.cash, value.debt
        let cash = parse_u64(
            fields
                .pointer("/value/fields/cash")
                .unwrap_or(&Value::Null),
        );
        let debt = parse_u64(
            fields
                .pointer("/value/fields/debt")
                .unwrap_or(&Value::Null),
        );
        if cash > 0 || debt > 0 {
            return Some((cash, debt));
        }

        // Fallback: direct balance_sheet path
        let cash2 = parse_u64(
            fields
                .pointer("/balance_sheet/fields/cash")
                .unwrap_or(&Value::Null),
        );
        let debt2 = parse_u64(
            fields
                .pointer("/balance_sheet/fields/debt")
                .unwrap_or(&Value::Null),
        );
        if cash2 > 0 || debt2 > 0 {
            return Some((cash2, debt2));
        }
    }
    None
}

pub async fn run(args: &MarketsArgs) -> Result<()> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    // Fetch pool addresses from Scallop API
    let pools = fetch_pool_addresses(&client).await?;

    // Filter if asset specified
    let asset_filter = args.asset.as_ref().map(|a| a.to_lowercase());

    let mut market_rows: Vec<serde_json::Value> = Vec::new();

    // Focus on the most liquid assets for display
    let priority_assets = vec![
        "sui", "usdc", "sca", "deep", "hasui", "cetus", "weth", "wsol",
        "sbeth", "sbusdt", "sbwbtc", "afsui", "vsui", "usdy", "wal", "ns",
    ];

    let mut ordered_keys: Vec<String> = priority_assets
        .iter()
        .filter(|k| pools.contains_key(**k))
        .map(|k| k.to_string())
        .collect();

    // Add any remaining assets
    for k in pools.keys() {
        if !ordered_keys.contains(k) {
            ordered_keys.push(k.clone());
        }
    }

    for key in &ordered_keys {
        // Apply filter
        if let Some(ref filter) = asset_filter {
            if !key.contains(filter.as_str())
                && !pools[key]
                    .symbol
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(filter.as_str())
            {
                continue;
            }
        }

        let pool = &pools[key];
        let symbol = pool
            .symbol
            .clone()
            .unwrap_or_else(|| key.to_uppercase());
        let decimals = pool.decimals.unwrap_or(9);

        // Attempt to fetch on-chain pool stats
        let (cash, debt) = if let Some(addr) = &pool.lending_pool_address {
            fetch_pool_stats(&client, addr, decimals)
                .await
                .unwrap_or((0, 0))
        } else {
            (0, 0)
        };

        let decimal_factor = 10u64.pow(decimals as u32) as f64;
        let cash_human = cash as f64 / decimal_factor;
        let debt_human = debt as f64 / decimal_factor;
        let total = cash_human + debt_human;
        let util = utilization(cash, debt);
        let supply_apy = approx_supply_apy(util);
        let borrow_apy = approx_borrow_apy(util);

        let row = serde_json::json!({
            "asset": symbol,
            "coin_name": key,
            "total_supplied": format!("{:.2}", total),
            "available_liquidity": format!("{:.2}", cash_human),
            "total_borrowed": format!("{:.2}", debt_human),
            "utilization_pct": format!("{:.1}", util * 100.0),
            "supply_apy_pct": format!("{:.2}", supply_apy * 100.0),
            "borrow_apy_pct": format!("{:.2}", borrow_apy * 100.0),
            "lending_pool": pool.lending_pool_address.clone().unwrap_or_default(),
        });
        market_rows.push(row);
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&market_rows)?);
        return Ok(());
    }

    // Table output
    println!("\nScallop Lend — Lending Markets (Sui Mainnet)");
    println!("{}", "=".repeat(110));
    println!(
        "{:<10} {:>16} {:>16} {:>16} {:>12} {:>12} {:>12}",
        "Asset", "Total Supply", "Available", "Total Borrowed", "Util%", "Supply APY", "Borrow APY"
    );
    println!("{}", "-".repeat(110));

    for row in &market_rows {
        println!(
            "{:<10} {:>16} {:>16} {:>16} {:>11}% {:>11}% {:>11}%",
            row["asset"].as_str().unwrap_or(""),
            row["total_supplied"].as_str().unwrap_or("0"),
            row["available_liquidity"].as_str().unwrap_or("0"),
            row["total_borrowed"].as_str().unwrap_or("0"),
            row["utilization_pct"].as_str().unwrap_or("0"),
            row["supply_apy_pct"].as_str().unwrap_or("0"),
            row["borrow_apy_pct"].as_str().unwrap_or("0"),
        );
    }

    println!("{}", "-".repeat(110));
    println!("Data: Sui JSON-RPC + Scallop API | APY: estimated from utilization model");
    println!("Protocol: https://scallop.io | Docs: https://developers.scallop.io");

    Ok(())
}
