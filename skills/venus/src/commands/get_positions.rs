// Venus — get-positions command

use crate::{config, onchainos, rpc};
use anyhow::Result;

pub async fn execute(chain_id: u64, wallet: Option<String>) -> Result<()> {
    let rpc_url = config::get_rpc(chain_id)?;

    // Resolve wallet
    let wallet_addr = match wallet {
        Some(w) => w,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Get all markets
    let markets = rpc::get_all_markets(rpc_url, config::COMPTROLLER).await?;

    let mut positions = Vec::new();
    let mut total_supply_usd = 0f64;
    let mut total_borrow_usd = 0f64;

    for vtoken_addr in &markets {
        let (err_code, vtoken_bal, borrow_bal, exchange_rate) =
            rpc::get_account_snapshot(rpc_url, vtoken_addr, &wallet_addr).await?;

        // Skip markets with no position
        if err_code != 0 || (vtoken_bal == 0 && borrow_bal == 0) {
            continue;
        }

        let sym = rpc::erc20_symbol(rpc_url, vtoken_addr).await;

        // Convert vToken balance to underlying
        // underlying = vTokenBalance * exchangeRate / 1e18
        // Note: exchangeRate has 18 + (underlying_decimals - vToken_decimals) decimals
        // For simplicity: underlying_amount_raw = vtoken_bal * exchange_rate / 1e18
        let supply_raw = if exchange_rate > 0 {
            (vtoken_bal as u128 * exchange_rate as u128) / 1_000_000_000_000_000_000u128
        } else {
            0
        };

        positions.push(serde_json::json!({
            "symbol": sym,
            "vtoken_address": vtoken_addr,
            "vtoken_balance_raw": vtoken_bal.to_string(),
            "supply_underlying_raw": supply_raw.to_string(),
            "borrow_balance_raw": borrow_bal.to_string(),
            "exchange_rate_raw": exchange_rate.to_string()
        }));

        let _ = (total_supply_usd, total_borrow_usd);
    }

    // Get account liquidity / health
    let (_, liquidity, shortfall) =
        rpc::get_account_liquidity(rpc_url, config::COMPTROLLER, &wallet_addr).await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "wallet": wallet_addr,
            "positions": positions,
            "account_liquidity_raw": liquidity.to_string(),
            "account_shortfall_raw": shortfall.to_string()
        }))?
    );

    Ok(())
}
