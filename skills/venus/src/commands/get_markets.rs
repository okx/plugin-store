// Venus — get-markets command

use crate::{config, rpc};
use anyhow::Result;

pub async fn execute(chain_id: u64) -> Result<()> {
    let rpc_url = config::get_rpc(chain_id)?;

    // Get all market addresses
    let markets = rpc::get_all_markets(rpc_url, config::COMPTROLLER).await?;

    let mut results = Vec::new();

    for vtoken_addr in &markets {
        // Get symbol
        let sym = rpc::erc20_symbol(rpc_url, vtoken_addr).await;

        // Get rates
        let supply_rate = rpc::get_supply_rate_per_block(rpc_url, vtoken_addr).await;
        let borrow_rate = rpc::get_borrow_rate_per_block(rpc_url, vtoken_addr).await;
        let total_borrows = rpc::get_total_borrows(rpc_url, vtoken_addr).await;
        let cash = rpc::get_cash(rpc_url, vtoken_addr).await;
        let exchange_rate = rpc::get_exchange_rate(rpc_url, vtoken_addr).await;

        // Compute APY
        let supply_apy = rpc::rate_to_apy(supply_rate, config::BLOCKS_PER_YEAR);
        let borrow_apy = rpc::rate_to_apy(borrow_rate, config::BLOCKS_PER_YEAR);

        // Get underlying symbol (for display)
        let underlying_addr = rpc::get_underlying(rpc_url, vtoken_addr).await;
        let underlying_sym = if &underlying_addr == "0x0000000000000000000000000000000000000000" {
            "BNB".to_string() // vBNB has no underlying() function
        } else {
            rpc::erc20_symbol(rpc_url, &underlying_addr).await
        };

        results.push(serde_json::json!({
            "symbol": sym,
            "vtoken_address": vtoken_addr,
            "underlying_symbol": underlying_sym,
            "underlying_address": underlying_addr,
            "supply_apy_pct": format!("{:.4}", supply_apy),
            "borrow_apy_pct": format!("{:.4}", borrow_apy),
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
            "market_count": results.len(),
            "markets": results
        }))?
    );

    Ok(())
}
