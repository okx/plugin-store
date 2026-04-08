// Segment Finance — get-positions command
// Shows current supply and borrow positions for a wallet

use crate::{config, onchainos, rpc};
use anyhow::Result;

pub async fn execute(chain_id: u64, wallet: Option<String>) -> Result<()> {
    let rpc_url = config::get_rpc(chain_id)?;

    // Resolve wallet
    let wallet_addr = match wallet {
        Some(w) => w,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Use known markets (reliable on Diamond proxy)
    let markets = rpc::get_known_markets();

    let mut positions = Vec::new();

    for setoken_addr in &markets {
        let (err_code, setoken_bal, borrow_bal, exchange_rate) =
            rpc::get_account_snapshot(rpc_url, setoken_addr, &wallet_addr).await?;

        // Skip markets with no position
        if err_code != 0 || (setoken_bal == 0 && borrow_bal == 0) {
            continue;
        }

        let sym = rpc::erc20_symbol(rpc_url, setoken_addr).await;
        let underlying_addr = rpc::get_underlying(rpc_url, setoken_addr).await;
        let underlying_sym = if underlying_addr == "0x0000000000000000000000000000000000000000" {
            "BNB".to_string()
        } else {
            rpc::erc20_symbol(rpc_url, &underlying_addr).await
        };

        // Underlying supply = seTokenBalance * exchangeRate / 1e18
        let supply_raw = if exchange_rate > 0 {
            (setoken_bal as u128)
                .saturating_mul(exchange_rate as u128)
                / 1_000_000_000_000_000_000u128
        } else {
            0
        };

        // Get USD price
        let price_raw = rpc::get_underlying_price(rpc_url, config::ORACLE, setoken_addr).await;
        let price_usd = price_raw as f64 / 1e18;
        let supply_usd = supply_raw as f64 / 1e18 * price_usd;
        let borrow_usd = borrow_bal as f64 / 1e18 * price_usd;

        positions.push(serde_json::json!({
            "symbol": sym,
            "underlying_symbol": underlying_sym,
            "setoken_address": setoken_addr,
            "setoken_balance_raw": setoken_bal.to_string(),
            "supply_underlying_raw": supply_raw.to_string(),
            "supply_usd": format!("{:.4}", supply_usd),
            "borrow_balance_raw": borrow_bal.to_string(),
            "borrow_usd": format!("{:.4}", borrow_usd),
            "exchange_rate_raw": exchange_rate.to_string()
        }));
    }

    // Get account health
    let (_, liquidity, shortfall) =
        rpc::get_account_liquidity(rpc_url, config::COMPTROLLER, &wallet_addr).await?;

    let health_status = if shortfall > 0 {
        "AT_RISK"
    } else if liquidity > 0 {
        "HEALTHY"
    } else {
        "NO_POSITION"
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "protocol": "Segment Finance",
            "wallet": wallet_addr,
            "positions": positions,
            "health_status": health_status,
            "account_liquidity_raw": liquidity.to_string(),
            "account_shortfall_raw": shortfall.to_string()
        }))?
    );

    Ok(())
}
