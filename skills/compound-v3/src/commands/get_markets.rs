use crate::config::get_market_config;
use crate::rpc;
use anyhow::Result;

pub async fn run(chain_id: u64, market: &str) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;

    let utilization = rpc::get_utilization(cfg.comet_proxy, cfg.rpc_url).await?;
    let supply_rate = rpc::get_supply_rate(cfg.comet_proxy, utilization, cfg.rpc_url).await?;
    let borrow_rate = rpc::get_borrow_rate(cfg.comet_proxy, utilization, cfg.rpc_url).await?;
    let total_supply = rpc::get_total_supply(cfg.comet_proxy, cfg.rpc_url).await?;
    let total_borrow = rpc::get_total_borrow(cfg.comet_proxy, cfg.rpc_url).await?;

    let supply_apr = rpc::rate_to_apr_pct(supply_rate);
    let borrow_apr = rpc::rate_to_apr_pct(borrow_rate);
    let util_pct = (utilization as f64 / 1e18) * 100.0;
    let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "base_asset": cfg.base_asset_symbol,
            "comet_proxy": cfg.comet_proxy,
            "utilization_pct": format!("{:.2}", util_pct),
            "supply_apr_pct": format!("{:.4}", supply_apr),
            "borrow_apr_pct": format!("{:.4}", borrow_apr),
            "total_supply": format!("{:.2}", total_supply as f64 / decimals_factor),
            "total_borrow": format!("{:.2}", total_borrow as f64 / decimals_factor),
            "total_supply_raw": total_supply.to_string(),
            "total_borrow_raw": total_borrow.to_string()
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
