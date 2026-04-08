use anyhow::Result;
use clap::Args;
use reqwest::Client;
use serde_json::Value;

use crate::api::fetch_pool_addresses;
use crate::config::PROTOCOL_PACKAGE;
use crate::rpc::{get_all_balances, get_object, get_owned_objects};

#[derive(Args, Debug)]
pub struct PositionsArgs {
    /// Sui wallet address to query
    #[arg(long)]
    pub wallet: String,

    /// Output raw JSON
    #[arg(long)]
    pub json: bool,
}

fn parse_u64(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n.as_u64().unwrap_or(0),
        Value::String(s) => s.parse::<u64>().unwrap_or(0),
        _ => 0,
    }
}

pub async fn run(args: &PositionsArgs) -> Result<()> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    let wallet = &args.wallet;

    // Validate address format
    if !wallet.starts_with("0x") || wallet.len() < 10 {
        anyhow::bail!("Invalid Sui address format: {}", wallet);
    }

    println!("Querying positions for: {}", wallet);

    // 1) Fetch all balances (sCoin holdings = supplied positions)
    let balances = get_all_balances(&client, wallet).await?;

    // 2) Fetch pool addresses to map sCoin types to asset names
    let pools = fetch_pool_addresses(&client).await?;

    // Build sCoinType -> pool mapping
    let mut scoin_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (coin_name, pool) in &pools {
        if let Some(ref scoin_type) = pool.scoin_symbol {
            scoin_map.insert(scoin_type.to_lowercase(), coin_name.clone());
        }
        if let Some(ref scoin_type) = pool.coin_type {
            scoin_map.insert(scoin_type.to_lowercase(), coin_name.clone());
        }
    }

    // 3) Look for Obligation objects (borrow positions)
    // Obligation type: PROTOCOL_PACKAGE::obligation::Obligation
    let obligation_type = format!("{}::obligation::Obligation", PROTOCOL_PACKAGE);
    let owned = get_owned_objects(&client, wallet, Some(&obligation_type)).await;

    let mut supply_positions: Vec<serde_json::Value> = Vec::new();
    let mut borrow_positions: Vec<serde_json::Value> = Vec::new();

    // Parse balance entries for sCoin holdings
    if let Value::Array(balance_arr) = &balances {
        for b in balance_arr {
            let coin_type = b["coinType"].as_str().unwrap_or("").to_lowercase();
            let total_balance: u64 = b["totalBalance"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);

            if total_balance == 0 {
                continue;
            }

            // Check if this is a Scallop sCoin (starts with known scallop package prefix)
            // sCoin types contain "scallop" in their module name
            if coin_type.contains("::scallop_") || coin_type.contains("scallop") {
                // Try to match to a pool
                let asset_name = pools
                    .iter()
                    .find(|(_, p)| {
                        p.scoin_symbol
                            .as_deref()
                            .map(|s| coin_type.contains(&s.to_lowercase()))
                            .unwrap_or(false)
                            || p.coin_name
                                .as_deref()
                                .map(|s| coin_type.contains(s))
                                .unwrap_or(false)
                    })
                    .map(|(k, _)| k.as_str())
                    .unwrap_or("unknown");

                let pool_info = pools.get(asset_name);
                let decimals = pool_info.and_then(|p| p.decimals).unwrap_or(9);
                let decimal_factor = 10u64.pow(decimals as u32) as f64;
                let amount_human = total_balance as f64 / decimal_factor;

                supply_positions.push(serde_json::json!({
                    "asset": pool_info
                        .and_then(|p| p.symbol.as_deref())
                        .unwrap_or(asset_name)
                        .to_uppercase(),
                    "scoin_type": coin_type,
                    "balance_raw": total_balance.to_string(),
                    "balance_human": format!("{:.6}", amount_human),
                    "type": "supply",
                }));
            }
        }
    }

    // Parse obligation objects for borrow positions
    if let Ok(owned_obj) = owned {
        if let Some(arr) = owned_obj["data"].as_array() {
            for item in arr {
                let obj_id = item["data"]["objectId"].as_str().unwrap_or("");
                if obj_id.is_empty() {
                    continue;
                }

                // Fetch obligation details
                if let Ok(obj) = get_object(&client, obj_id).await {
                    let fields = obj.pointer("/data/content/fields");
                    if let Some(f) = fields {
                        // Extract borrow positions from debt_bag
                        // obligations store collaterals and debts
                        let debt_value = parse_u64(f.get("debt_value").unwrap_or(&Value::Null));
                        let collateral_value =
                            parse_u64(f.get("collateral_value").unwrap_or(&Value::Null));

                        if debt_value > 0 || collateral_value > 0 {
                            borrow_positions.push(serde_json::json!({
                                "obligation_id": obj_id,
                                "debt_value_usd_raw": debt_value.to_string(),
                                "collateral_value_usd_raw": collateral_value.to_string(),
                                "health_factor": if debt_value > 0 {
                                    format!("{:.3}", collateral_value as f64 / debt_value as f64)
                                } else {
                                    "N/A".to_string()
                                },
                            }));
                        }
                    }
                }
            }
        }
    }

    let result = serde_json::json!({
        "wallet": wallet,
        "supply_positions": supply_positions,
        "borrow_positions": borrow_positions,
        "note": "Supply positions shown as sCoin holdings. Borrow positions from Obligation objects.",
        "source": "Sui JSON-RPC (fullnode.mainnet.sui.io)"
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    println!("\nScallop Lend — Positions for {}", wallet);
    println!("{}", "=".repeat(80));

    println!("\n-- Supply Positions (sCoin holdings) --");
    if supply_positions.is_empty() {
        println!("  No Scallop supply positions found (no sCoin tokens in wallet).");
    } else {
        println!("{:<12} {:>20} {}", "Asset", "Balance", "sCoin Type");
        println!("{}", "-".repeat(80));
        for pos in &supply_positions {
            println!(
                "{:<12} {:>20} {}",
                pos["asset"].as_str().unwrap_or(""),
                pos["balance_human"].as_str().unwrap_or("0"),
                pos["scoin_type"].as_str().unwrap_or(""),
            );
        }
    }

    println!("\n-- Borrow Positions (Obligations) --");
    if borrow_positions.is_empty() {
        println!("  No active Scallop obligation/borrow positions found.");
    } else {
        for pos in &borrow_positions {
            println!(
                "Obligation: {} | Debt Value: {} | Collateral Value: {} | Health: {}",
                pos["obligation_id"].as_str().unwrap_or(""),
                pos["debt_value_usd_raw"].as_str().unwrap_or("0"),
                pos["collateral_value_usd_raw"].as_str().unwrap_or("0"),
                pos["health_factor"].as_str().unwrap_or("N/A"),
            );
        }
    }

    println!("\n{}", "-".repeat(80));
    println!("Data source: Sui mainnet JSON-RPC");

    Ok(())
}
