// rates command — show exchange price and yield information for Instadapp Lite vaults

use crate::{config, rpc};
use anyhow::Result;
use serde_json::json;

pub async fn execute(chain_id: u64) -> Result<()> {
    let rpc_url = config::ETHEREUM_RPC;

    let mut rates_list = Vec::new();

    // iETH v1 rates
    let v1_addr = config::IETH_V1_VAULT;
    let (exchange_price_v1, new_revenue) = rpc::get_exchange_price_v1(v1_addr, rpc_url)
        .await
        .unwrap_or((1_000_000_000_000_000_000, 0));
    let total_supply_v1 = rpc::get_total_supply(v1_addr, rpc_url)
        .await
        .unwrap_or(0);
    let (net_collateral, net_borrow) = rpc::get_net_assets_v1(v1_addr, rpc_url)
        .await
        .unwrap_or((0, 0));

    let exchange_price_v1_f = exchange_price_v1 as f64 / 1e18;
    let cumulative_yield_pct = (exchange_price_v1_f - 1.0) * 100.0;
    let total_supply_f = total_supply_v1 as f64 / 1e18;
    let net_borrow_f = net_borrow as f64 / 1e18;
    let net_collateral_f = net_collateral as f64 / 1e18;
    let leverage_ratio = if net_borrow_f > 0.0 { net_collateral_f / net_borrow_f } else { 0.0 };

    rates_list.push(json!({
        "vault_address": v1_addr,
        "vault_name": "Instadapp ETH",
        "symbol": "iETH",
        "version": "v1",
        "underlying": "ETH",
        "exchange_price": format!("{:.6} ETH per iETH", exchange_price_v1_f),
        "cumulative_yield_pct": format!("{:.2}%", cumulative_yield_pct),
        "pending_revenue_eth": format!("{:.6}", new_revenue as f64 / 1e18),
        "total_supply_ieth": format!("{:.4}", total_supply_f),
        "net_collateral_eth": format!("{:.4}", net_collateral_f),
        "net_borrow_eth": format!("{:.4}", net_borrow_f),
        "leverage_ratio": if leverage_ratio > 0.0 { format!("{:.2}x", leverage_ratio) } else { "N/A".to_string() },
        "strategy": "Leveraged stETH/WETH yield via Aave V2/V3"
    }));

    // iETH v2 rates
    let v2_addr = config::IETH_V2_VAULT;
    let exchange_price_v2 = rpc::get_exchange_price_v2(v2_addr, rpc_url)
        .await
        .unwrap_or(1_000_000_000_000_000_000);
    let total_assets_v2 = rpc::get_total_assets(v2_addr, rpc_url)
        .await
        .unwrap_or(0);
    let total_supply_v2 = rpc::get_total_supply(v2_addr, rpc_url)
        .await
        .unwrap_or(0);

    let exchange_price_v2_f = exchange_price_v2 as f64 / 1e18;
    let cumulative_yield_v2 = (exchange_price_v2_f - 1.0) * 100.0;
    let total_assets_f = total_assets_v2 as f64 / 1e18;
    let total_supply_v2_f = total_supply_v2 as f64 / 1e18;

    rates_list.push(json!({
        "vault_address": v2_addr,
        "vault_name": "Instadapp ETH v2",
        "symbol": "iETHv2",
        "version": "v2",
        "underlying": "stETH",
        "exchange_price": format!("{:.6} stETH per iETHv2", exchange_price_v2_f),
        "cumulative_yield_pct": format!("{:.2}%", cumulative_yield_v2),
        "total_assets_steth": format!("{:.4}", total_assets_f),
        "total_supply_iethv2": format!("{:.4}", total_supply_v2_f),
        "strategy": "stETH yield aggregated across Aave V3, Compound V3, Spark, Fluid (ERC-4626)"
    }));

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "chain_id": chain_id,
                "rates": rates_list,
                "note": "Exchange price starts at 1.0 and grows as yield accrues. Cumulative yield since vault inception."
            }
        }))?
    );
    Ok(())
}
