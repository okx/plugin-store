use crate::config::get_market_config;
use crate::onchainos;
use crate::rpc;
use anyhow::Result;

pub async fn run(chain_id: u64, market: &str, wallet: Option<String>, collateral_asset: Option<String>) -> Result<()> {
    let cfg = get_market_config(chain_id, market)?;

    let wallet_addr = match wallet {
        Some(w) => w,
        None => {
            let w = onchainos::resolve_wallet(chain_id)?;
            if w.is_empty() {
                anyhow::bail!("Cannot resolve wallet address. Pass --wallet or log in via onchainos.");
            }
            w
        }
    };

    let supply_balance = rpc::get_balance_of(cfg.comet_proxy, &wallet_addr, cfg.rpc_url).await?;
    let borrow_balance = rpc::get_borrow_balance_of(cfg.comet_proxy, &wallet_addr, cfg.rpc_url).await?;
    let is_collateralized = rpc::is_borrow_collateralized(cfg.comet_proxy, &wallet_addr, cfg.rpc_url).await?;

    let decimals_factor = 10u128.pow(cfg.base_asset_decimals as u32) as f64;

    let mut collateral_info = serde_json::json!(null);
    if let Some(asset) = &collateral_asset {
        let col_bal = rpc::get_collateral_balance_of(cfg.comet_proxy, &wallet_addr, asset, cfg.rpc_url).await?;
        let col_decimals = rpc::get_erc20_decimals(asset, cfg.rpc_url).await.unwrap_or(18);
        let col_factor = 10u128.pow(col_decimals as u32) as f64;
        collateral_info = serde_json::json!({
            "asset": asset,
            "balance": format!("{:.decimals$}", col_bal as f64 / col_factor, decimals = col_decimals as usize),
            "balance_raw": col_bal.to_string(),
        });
    }

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "chain_id": chain_id,
            "market": market,
            "base_asset": cfg.base_asset_symbol,
            "wallet": wallet_addr,
            "supply_balance": format!("{:.6}", supply_balance as f64 / decimals_factor),
            "supply_balance_raw": supply_balance.to_string(),
            "borrow_balance": format!("{:.6}", borrow_balance as f64 / decimals_factor),
            "borrow_balance_raw": borrow_balance.to_string(),
            "is_borrow_collateralized": is_collateralized,
            "collateral": collateral_info
        }
    });

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
