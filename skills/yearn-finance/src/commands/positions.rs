// positions command — query user's Yearn vault holdings
// Uses concurrent RPC calls for efficiency

use crate::{api, config, onchainos, rpc};
use anyhow::Result;
use serde_json::json;

pub async fn execute(
    chain_id: u64,
    wallet_override: Option<&str>,
) -> Result<()> {
    // Resolve wallet address
    let wallet = if let Some(w) = wallet_override {
        w.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let rpc_url = if chain_id == 1 {
        config::ETHEREUM_RPC
    } else {
        "https://ethereum.publicnode.com"
    };

    let vaults = api::fetch_vaults(chain_id).await?;
    // Sort by TVL descending and cap at top 50 vaults to keep RPC usage manageable
    let mut sorted_vaults: Vec<_> = vaults.iter().filter(|v| v.is_active()).collect();
    sorted_vaults.sort_by(|a, b| {
        let ta = a.tvl.as_ref().and_then(|t| t.tvl).unwrap_or(0.0);
        let tb = b.tvl.as_ref().and_then(|t| t.tvl).unwrap_or(0.0);
        tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
    });
    let top_vaults: Vec<_> = sorted_vaults.into_iter().take(50).collect();

    // Concurrent balanceOf queries for all top vaults
    let wallet_clone = wallet.clone();
    let rpc_url_str = rpc_url.to_string();

    let balance_futs: Vec<_> = top_vaults.iter().map(|vault| {
        let addr = vault.address.clone();
        let w = wallet_clone.clone();
        let rpc = rpc_url_str.clone();
        tokio::spawn(async move {
            rpc::get_balance_of(&addr, &w, &rpc).await
        })
    }).collect();

    let balances: Vec<u128> = futures_util::future::join_all(balance_futs)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(Ok(0)).unwrap_or(0))
        .collect();

    let mut positions = Vec::new();

    for (vault, shares) in top_vaults.iter().zip(balances.iter()) {
        if *shares == 0 {
            continue;
        }

        let decimals = vault.decimals.unwrap_or(18) as u32;
        let token_decimals = vault.token.decimals;

        // Query pricePerShare for vaults where user has shares
        let price_per_share = rpc::get_price_per_share(&vault.address, rpc_url)
            .await
            .unwrap_or(10u128.pow(decimals));

        let underlying_raw = (*shares as u128)
            .saturating_mul(price_per_share)
            / 10u128.pow(decimals);

        let underlying_display = format!(
            "{:.6}",
            underlying_raw as f64 / 10f64.powi(token_decimals as i32)
        );

        let shares_display = format!(
            "{:.6}",
            *shares as f64 / 10f64.powi(decimals as i32)
        );

        positions.push(json!({
            "vault_address": vault.address,
            "vault_name": vault.name.as_deref().unwrap_or("Unknown"),
            "vault_symbol": vault.symbol.as_deref().unwrap_or(""),
            "token": vault.token.symbol,
            "token_address": vault.token.address,
            "shares": shares_display,
            "underlying_balance": underlying_display,
            "net_apr": vault.apr_display(),
            "tvl_usd": vault.tvl_display()
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
                "note": "Scans top 50 vaults by TVL. For full history use --vault flag."
            }
        }))?
    );
    Ok(())
}
