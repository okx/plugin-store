// positions command — query user's Instadapp Lite vault holdings

use crate::{config, onchainos, rpc};
use anyhow::Result;
use serde_json::json;

pub async fn execute(chain_id: u64, wallet_override: Option<&str>) -> Result<()> {
    // Resolve wallet address
    let wallet = if let Some(w) = wallet_override {
        w.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let rpc_url = config::ETHEREUM_RPC;
    let mut positions = Vec::new();

    // Check iETH v1 position
    let v1_addr = config::IETH_V1_VAULT;
    let v1_shares = rpc::get_balance_of(v1_addr, &wallet, rpc_url)
        .await
        .unwrap_or(0);

    if v1_shares > 0 {
        let (exchange_price, _) = rpc::get_exchange_price_v1(v1_addr, rpc_url)
            .await
            .unwrap_or((1_000_000_000_000_000_000, 0));

        let underlying_raw = (v1_shares as u128)
            .saturating_mul(exchange_price)
            / 1_000_000_000_000_000_000u128;

        positions.push(json!({
            "vault_address": v1_addr,
            "vault_name": "Instadapp ETH",
            "symbol": "iETH",
            "version": "v1",
            "shares": format!("{:.6}", v1_shares as f64 / 1e18),
            "shares_raw": v1_shares.to_string(),
            "underlying_eth": format!("{:.6}", underlying_raw as f64 / 1e18),
            "exchange_price": format!("{:.6}", exchange_price as f64 / 1e18),
            "withdraw_note": "Use 'instadapp withdraw --vault v1 --shares <amount>' to exit"
        }));
    }

    // Check iETH v2 position
    let v2_addr = config::IETH_V2_VAULT;
    let v2_shares = rpc::get_balance_of(v2_addr, &wallet, rpc_url)
        .await
        .unwrap_or(0);

    if v2_shares > 0 {
        let exchange_price_v2 = rpc::get_exchange_price_v2(v2_addr, rpc_url)
            .await
            .unwrap_or(1_000_000_000_000_000_000);

        let underlying_steth = (v2_shares as u128)
            .saturating_mul(exchange_price_v2)
            / 1_000_000_000_000_000_000u128;

        positions.push(json!({
            "vault_address": v2_addr,
            "vault_name": "Instadapp ETH v2",
            "symbol": "iETHv2",
            "version": "v2",
            "shares": format!("{:.6}", v2_shares as f64 / 1e18),
            "shares_raw": v2_shares.to_string(),
            "underlying_steth": format!("{:.6}", underlying_steth as f64 / 1e18),
            "exchange_price": format!("{:.6}", exchange_price_v2 as f64 / 1e18),
            "withdraw_note": "Use 'instadapp withdraw --vault v2 --shares <amount>' to exit"
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "chain_id": chain_id,
                "position_count": positions.len(),
                "positions": positions,
                "note": if positions.is_empty() {
                    "No Instadapp Lite positions found. Use 'instadapp deposit' to start."
                } else {
                    "Instadapp Lite positions found."
                }
            }
        }))?
    );
    Ok(())
}
