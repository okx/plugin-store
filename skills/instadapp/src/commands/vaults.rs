// vaults command — list Instadapp Lite vaults with exchange price and TVL
// Queries iETH v1 and iETHv2 vaults directly via on-chain RPC

use crate::{config, rpc};
use anyhow::Result;
use serde_json::json;

pub async fn execute(chain_id: u64) -> Result<()> {
    let rpc_url = if chain_id == 1 {
        config::ETHEREUM_RPC
    } else {
        config::ETHEREUM_RPC // Lite vaults are Ethereum-only
    };

    let mut vault_list = Vec::new();

    // Query iETH v1 vault
    let v1_addr = config::IETH_V1_VAULT;
    let (exchange_price_v1, _revenue) = rpc::get_exchange_price_v1(v1_addr, rpc_url)
        .await
        .unwrap_or((1_000_000_000_000_000_000, 0));
    let total_supply_v1 = rpc::get_total_supply(v1_addr, rpc_url)
        .await
        .unwrap_or(0);
    let (net_collateral, net_borrow) = rpc::get_net_assets_v1(v1_addr, rpc_url)
        .await
        .unwrap_or((0, 0));

    let exchange_price_v1_f = exchange_price_v1 as f64 / 1e18;
    let total_supply_v1_f = total_supply_v1 as f64 / 1e18;
    let net_collateral_eth = net_collateral as f64 / 1e18;
    let net_borrow_eth = net_borrow as f64 / 1e18;
    // TVL approximation: total_supply * exchange_price
    let tvl_eth_v1 = total_supply_v1_f * exchange_price_v1_f;

    vault_list.push(json!({
        "address": v1_addr,
        "name": "Instadapp ETH",
        "symbol": "iETH",
        "version": "v1",
        "underlying": "ETH",
        "deposit_method": "supplyEth(address) — send ETH directly",
        "exchange_price_eth": format!("{:.6}", exchange_price_v1_f),
        "total_supply": format!("{:.4}", total_supply_v1_f),
        "tvl_eth": format!("{:.4}", tvl_eth_v1),
        "net_collateral_eth": format!("{:.4}", net_collateral_eth),
        "net_borrow_eth": format!("{:.4}", net_borrow_eth),
        "note": "Leveraged ETH vault using stETH/WETH. Exchange price grows as yield accrues."
    }));

    // Query iETH v2 vault
    let v2_addr = config::IETH_V2_VAULT;
    let exchange_price_v2 = rpc::get_exchange_price_v2(v2_addr, rpc_url)
        .await
        .unwrap_or(1_000_000_000_000_000_000);
    let total_supply_v2 = rpc::get_total_supply(v2_addr, rpc_url)
        .await
        .unwrap_or(0);
    let total_assets_v2 = rpc::get_total_assets(v2_addr, rpc_url)
        .await
        .unwrap_or(0);

    let exchange_price_v2_f = exchange_price_v2 as f64 / 1e18;
    let total_supply_v2_f = total_supply_v2 as f64 / 1e18;
    let total_assets_v2_f = total_assets_v2 as f64 / 1e18;

    vault_list.push(json!({
        "address": v2_addr,
        "name": "Instadapp ETH v2",
        "symbol": "iETHv2",
        "version": "v2",
        "underlying": "stETH",
        "deposit_method": "ERC-4626 deposit(uint256,address) — requires stETH approval",
        "exchange_price_steth": format!("{:.6}", exchange_price_v2_f),
        "total_supply": format!("{:.4}", total_supply_v2_f),
        "total_assets_steth": format!("{:.4}", total_assets_v2_f),
        "note": "ERC-4626 vault. Deposit stETH to receive iETHv2 shares. Aggregates yield across Aave V3, Compound, Spark, Fluid."
    }));

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "chain_id": chain_id,
                "count": vault_list.len(),
                "vaults": vault_list
            }
        }))?
    );
    Ok(())
}
