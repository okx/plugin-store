// commands/get_balances.rs — Query user LP token balances across Curve pools
use crate::{api, config, multicall, onchainos, rpc};
use anyhow::Result;

pub async fn run(chain_id: u64, wallet: Option<String>) -> Result<()> {
    let chain_name = config::chain_name(chain_id);
    let rpc_url = config::rpc_url(chain_id);

    // Resolve wallet address
    let wallet_addr = match wallet {
        Some(w) => w,
        None => {
            let w = onchainos::resolve_wallet(chain_id)?;
            if w.is_empty() {
                anyhow::bail!("Cannot determine wallet address. Pass --wallet or ensure onchainos is logged in.");
            }
            w
        }
    };

    // Fetch all pools
    let pools = api::get_all_pools(chain_name).await?;

    // Round 1 — Multicall: pool.token() for every pool to get LP token addresses.
    // Factory-crypto pools have a separate LP token contract; others return pool address or fail.
    let token_calls: Vec<(String, Vec<u8>)> = pools
        .iter()
        .map(|p| (p.address.clone(), hex::decode("fc0c546a").unwrap()))
        .collect();
    let token_results = multicall::batch_call(token_calls, rpc_url)
        .await
        .unwrap_or_default();

    let lp_tokens: Vec<String> = pools
        .iter()
        .zip(token_results.iter())
        .map(|(pool, res)| match res {
            Some(data) => multicall::decode_address(data, &pool.address),
            None => pool.address.clone(),
        })
        .collect();

    // Round 2 — Multicall: balanceOf(wallet) for every LP token.
    let wallet_clean = wallet_addr.trim_start_matches("0x");
    let mut wallet_word = vec![0u8; 32];
    let wb = hex::decode(wallet_clean).unwrap_or_default();
    wallet_word[32 - wb.len()..].copy_from_slice(&wb);

    let balance_calls: Vec<(String, Vec<u8>)> = lp_tokens
        .iter()
        .map(|lp| {
            let mut cd = hex::decode("70a08231").unwrap();
            cd.extend_from_slice(&wallet_word);
            (lp.clone(), cd)
        })
        .collect();
    let balance_results = multicall::batch_call(balance_calls, rpc_url)
        .await
        .unwrap_or_default();

    // Build positions (only pools with non-zero LP balance)
    let mut positions = Vec::new();
    for ((pool, lp_token), bal_res) in pools
        .iter()
        .zip(lp_tokens.iter())
        .zip(balance_results.iter())
    {
        let balance = bal_res
            .as_deref()
            .map(multicall::decode_u128)
            .unwrap_or(0);
        if balance == 0 {
            continue;
        }
        // decimals() only for pools we actually hold — usually 0–2 calls
        let dec = rpc::decimals(lp_token, rpc_url).await;
        let lp_balance = balance as f64 / 10f64.powi(dec as i32);
        let coins: Vec<_> = pool.coins.iter().map(|c| c.symbol.as_str()).collect();
        positions.push(serde_json::json!({
            "pool_id": pool.id,
            "pool_name": pool.name,
            "pool_address": pool.address,
            "lp_token_address": lp_token,
            "coins": coins,
            "lp_balance": format!("{:.6}", lp_balance),
            "lp_balance_raw": balance.to_string(),
            "tvl_usd": pool.usd_total
        }));
    }

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "wallet": wallet_addr,
            "chain": chain_name,
            "positions_count": positions.len(),
            "positions": positions
        })
    );
    Ok(())
}
