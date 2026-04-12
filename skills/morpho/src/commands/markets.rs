use crate::api;
use crate::config::chain_name;

/// List Morpho Blue markets with APYs, optionally filtered by asset.
pub async fn run(chain_id: u64, asset_filter: Option<&str>) -> anyhow::Result<()> {
    let markets = api::list_markets(chain_id, asset_filter).await?;

    // Threshold above which an APY is considered anomalous (e.g. expired Pendle PT collateral)
    const APY_ANOMALY_THRESHOLD: f64 = 5.0; // 500%

    let items: Vec<serde_json::Value> = markets.iter().map(|m| {
        let loan_symbol = m.loan_asset.as_ref().map(|a| a.symbol.as_str()).unwrap_or("?");
        let collateral_symbol = m.collateral_asset.as_ref().map(|a| a.symbol.as_str()).unwrap_or("?");
        let supply_apy = m.state.as_ref().and_then(|s| s.supply_apy).unwrap_or(0.0);
        let borrow_apy = m.state.as_ref().and_then(|s| s.borrow_apy).unwrap_or(0.0);
        let utilization = m.state.as_ref().and_then(|s| s.utilization).unwrap_or(0.0);
        let lltv = m.lltv.as_deref().unwrap_or("0");
        let lltv_val: f64 = lltv.parse::<u128>().unwrap_or(0) as f64 / 1e18 * 100.0;

        let apy_warning: Option<&str> = if supply_apy > APY_ANOMALY_THRESHOLD || borrow_apy > APY_ANOMALY_THRESHOLD {
            Some("APY exceeds 500% — likely an expired Pendle PT collateral position or stale data. Do not supply based on this APY alone; verify the market on-chain before proceeding.")
        } else {
            None
        };

        let mut entry = serde_json::json!({
            "marketId": m.unique_key,
            "loanAsset": loan_symbol,
            "collateralAsset": collateral_symbol,
            "lltv": format!("{:.1}%", lltv_val),
            "supplyApy": format!("{:.4}%", supply_apy * 100.0),
            "borrowApy": format!("{:.4}%", borrow_apy * 100.0),
            "utilization": format!("{:.2}%", utilization * 100.0),
        });
        if let Some(w) = apy_warning {
            entry["warning"] = serde_json::Value::String(w.to_string());
        }
        entry
    }).collect();

    let output = serde_json::json!({
        "ok": true,
        "chain": chain_name(chain_id),
        "chainId": chain_id,
        "marketCount": items.len(),
        "markets": items,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
