// Segment Finance — get-markets command
// Lists all seToken markets with supply/borrow APY and utilization

use crate::{config, rpc};
use anyhow::Result;

pub async fn execute(chain_id: u64) -> Result<()> {
    let rpc_url = config::get_rpc(chain_id)?;

    // Get all market addresses (falls back to known list if Diamond returns garbage)
    let markets = rpc::get_all_markets(rpc_url, config::COMPTROLLER).await?;

    let mut results = Vec::new();

    for setoken_addr in &markets {
        // Get seToken symbol
        let sym = rpc::erc20_symbol(rpc_url, setoken_addr).await;

        // Get rates
        let supply_rate = rpc::get_supply_rate_per_block(rpc_url, setoken_addr).await;
        let borrow_rate = rpc::get_borrow_rate_per_block(rpc_url, setoken_addr).await;
        let total_borrows = rpc::get_total_borrows(rpc_url, setoken_addr).await;
        let cash = rpc::get_cash(rpc_url, setoken_addr).await;
        let exchange_rate = rpc::get_exchange_rate_stored(rpc_url, setoken_addr).await;

        // Compute APY (simple linear approximation)
        let supply_apy = rpc::rate_to_apy(supply_rate, config::BLOCKS_PER_YEAR);
        let borrow_apy = rpc::rate_to_apy(borrow_rate, config::BLOCKS_PER_YEAR);

        // Get underlying info
        let underlying_addr = rpc::get_underlying(rpc_url, setoken_addr).await;
        let underlying_sym = if underlying_addr == "0x0000000000000000000000000000000000000000" {
            "BNB".to_string() // seBNB has no underlying() function
        } else {
            rpc::erc20_symbol(rpc_url, &underlying_addr).await
        };

        // Get USD price from oracle
        let price_raw = rpc::get_underlying_price(rpc_url, config::ORACLE, setoken_addr).await;
        let price_usd = price_raw as f64 / 1e18;

        results.push(serde_json::json!({
            "symbol": sym,
            "setoken_address": setoken_addr,
            "underlying_symbol": underlying_sym,
            "underlying_address": underlying_addr,
            "supply_apy_pct": format!("{:.4}", supply_apy),
            "borrow_apy_pct": format!("{:.4}", borrow_apy),
            "price_usd": format!("{:.4}", price_usd),
            "total_borrows_raw": total_borrows.to_string(),
            "total_cash_raw": cash.to_string(),
            "exchange_rate_raw": exchange_rate.to_string()
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "protocol": "Segment Finance",
            "market_count": results.len(),
            "markets": results
        }))?
    );

    Ok(())
}
