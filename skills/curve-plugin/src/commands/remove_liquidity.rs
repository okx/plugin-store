// commands/remove_liquidity.rs — Remove liquidity from a Curve pool
use crate::{api, config, curve_abi, onchainos, rpc};
use anyhow::Result;

pub async fn run(
    chain_id: u64,
    pool_address: String,
    lp_amount_str: Option<String>, // None means "all"; human-readable LP token amount (18 dec)
    coin_index: Option<i64>,       // None = proportional, Some(i) = single-coin
    min_amount_strs: Vec<String>,  // human-readable min amounts per coin
    wallet: Option<String>,
    dry_run: bool,
    confirm: bool,
) -> Result<()> {
    let chain_name = config::chain_name(chain_id);
    let rpc_url = config::rpc_url(chain_id);

    // Resolve wallet address — always fetch real address even in dry_run for accurate preview
    let wallet_addr = match wallet.clone() {
        Some(w) => w,
        None => {
            let w = onchainos::resolve_wallet(chain_id)?;
            if w.is_empty() {
                anyhow::bail!("Cannot determine wallet address. Pass --wallet or ensure onchainos is logged in.");
            }
            w
        }
    };

    // Fetch pool info first — needed to resolve LP token address for v1 pools
    let pools = api::get_all_pools(chain_name).await?;
    let pool = api::find_pool_by_address(&pools, &pool_address);

    // Resolve LP token address: v1 pools use a separate LP token contract;
    // factory/crypto pools use the pool address itself as the LP token.
    let lp_token_addr = pool
        .and_then(|p| p.lp_token_address.as_deref())
        .filter(|s| !s.is_empty())
        .unwrap_or(&pool_address);

    // Parse human-readable lp_amount if provided (LP tokens are always 18 decimals)
    let parsed_lp_amount: Option<u128> = match &lp_amount_str {
        Some(s) => Some(rpc::parse_human_amount(s, 18)?),
        None => None,
    };

    // Get LP balance. Skip live lookup for dry-run and preview (no --confirm) paths —
    // both just need a plausible amount to build calldata / estimate output.
    let lp_balance = if dry_run || !confirm {
        parsed_lp_amount.unwrap_or(1_000_000_000_000_000_000u128) // 1e18 placeholder
    } else {
        let bal = rpc::balance_of(lp_token_addr, &wallet_addr, rpc_url).await?;
        if bal == 0 {
            anyhow::bail!("No LP token balance for pool {}", pool_address);
        }
        bal
    };

    let actual_lp_amount = parsed_lp_amount.unwrap_or(lp_balance);
    let n_coins = pool.map(|p| p.coins.len()).unwrap_or(2);

    // Parse human-readable min_amounts using per-coin decimals
    let min_amounts: Vec<u128> = if let Some(p) = pool {
        let mut parsed = Vec::new();
        for (i, s) in min_amount_strs.iter().enumerate() {
            let coin_decimals: u8 = p
                .coins
                .get(i)
                .and_then(|c| c.decimals.as_deref())
                .and_then(|d| d.parse().ok())
                .unwrap_or(18);
            parsed.push(rpc::parse_human_amount(s, coin_decimals)?);
        }
        parsed
    } else {
        min_amount_strs
            .iter()
            .map(|s| rpc::parse_human_amount(s, 18))
            .collect::<Result<Vec<_>>>()?
    };

    // Build calldata
    let calldata = if let Some(idx) = coin_index {
        // Single-coin withdrawal
        let min_out = min_amounts.first().copied().unwrap_or(0);
        // For dry_run, estimate expected output
        if dry_run {
            let est_calldata = curve_abi::encode_calc_withdraw_one_coin(actual_lp_amount, idx);
            let est_hex = rpc::eth_call(&pool_address, &est_calldata, rpc_url)
                .await
                .unwrap_or_default();
            let estimated = rpc::decode_uint128(&est_hex);
            let coin_decimals: u8 = pool
                .and_then(|p| p.coins.get(idx as usize))
                .and_then(|c| c.decimals.as_deref())
                .and_then(|d| d.parse().ok())
                .unwrap_or(18);
            println!(
                "{}",
                serde_json::json!({
                    "ok": true,
                    "dry_run": true,
                    "chain": chain_name,
                    "pool_address": pool_address,
                    "lp_amount": format!("{:.6}", actual_lp_amount as f64 / 1e18),
                    "lp_amount_raw": actual_lp_amount.to_string(),
                    "coin_index": idx,
                    "estimated_out": format!("{:.6}", estimated as f64 / 10f64.powi(coin_decimals as i32)),
                    "estimated_out_raw": estimated.to_string(),
                    "min_amount": format!("{:.6}", min_out as f64 / 10f64.powi(coin_decimals as i32)),
                    "min_amount_raw": min_out.to_string()
                })
            );
            return Ok(());
        }
        curve_abi::encode_remove_liquidity_one_coin(actual_lp_amount, idx, min_out)
    } else {
        // Proportional withdrawal
        match n_coins {
            2 => {
                let mins = [
                    min_amounts.first().copied().unwrap_or(0),
                    min_amounts.get(1).copied().unwrap_or(0),
                ];
                curve_abi::encode_remove_liquidity_2(actual_lp_amount, mins)
            }
            3 => {
                let mins = [
                    min_amounts.first().copied().unwrap_or(0),
                    min_amounts.get(1).copied().unwrap_or(0),
                    min_amounts.get(2).copied().unwrap_or(0),
                ];
                curve_abi::encode_remove_liquidity_3(actual_lp_amount, mins)
            }
            4 => {
                let mins = [
                    min_amounts.first().copied().unwrap_or(0),
                    min_amounts.get(1).copied().unwrap_or(0),
                    min_amounts.get(2).copied().unwrap_or(0),
                    min_amounts.get(3).copied().unwrap_or(0),
                ];
                curve_abi::encode_remove_liquidity_4(actual_lp_amount, mins)
            }
            _ => anyhow::bail!("Unsupported pool coin count: {}", n_coins),
        }
    };

    // Confirm gate: show preview and exit if --confirm not given (and not dry-run)
    if !dry_run && !confirm {
        let pool_name = pool.map(|p| p.name.as_str()).unwrap_or("unknown");
        let min_amounts_raw: Vec<String> = min_amounts.iter().map(|a| a.to_string()).collect();
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "preview": true,
                "operation": "remove-liquidity",
                "chain": chain_name,
                "pool_address": pool_address,
                "pool_name": pool_name,
                "lp_amount": format!("{:.6}", actual_lp_amount as f64 / 1e18),
                "lp_amount_raw": actual_lp_amount.to_string(),
                "coin_index": coin_index,
                "min_amounts_raw": min_amounts_raw,
                "calldata": calldata,
                "note": "Re-run with --confirm to execute on-chain."
            })
        );
        return Ok(());
    }

    if dry_run {
        let pool_name = pool.map(|p| p.name.as_str()).unwrap_or("unknown");
        let min_amounts_raw: Vec<String> = min_amounts.iter().map(|a| a.to_string()).collect();
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain": chain_name,
                "pool_address": pool_address,
                "pool_name": pool_name,
                "lp_amount": format!("{:.6}", actual_lp_amount as f64 / 1e18),
                "lp_amount_raw": actual_lp_amount.to_string(),
                "coin_index": coin_index,
                "min_amounts_raw": min_amounts_raw,
                "calldata": calldata,
                "note": "Re-run with --confirm to execute on-chain."
            })
        );
        return Ok(());
    }

    // Snapshot coin balances before execution to report received amounts after confirmation
    let coin_addrs: Vec<String> = pool
        .map(|p| p.coins.iter().map(|c| c.address.clone()).collect())
        .unwrap_or_default();
    let mut balances_before: Vec<u128> = Vec::new();
    for addr in &coin_addrs {
        let b = rpc::balance_of(addr, &wallet_addr, rpc_url).await.unwrap_or(0);
        balances_before.push(b);
    }

    // Execute remove_liquidity — requires --force
    let result = onchainos::wallet_contract_call(
        chain_id,
        &pool_address,
        &calldata,
        Some(&wallet_addr),
        None,
        true,  // --force required
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;
    let explorer = config::explorer_url(chain_id, &tx_hash);
    let pool_name = pool.map(|p| p.name.as_str()).unwrap_or("unknown");

    let _ = onchainos::wait_for_tx(chain_id, tx_hash.clone(), wallet_addr.clone()).await;
    let mut received: Vec<serde_json::Value> = Vec::new();
    for (i, addr) in coin_addrs.iter().enumerate() {
        let coin_symbol = pool
            .and_then(|p| p.coins.get(i))
            .map(|c| c.symbol.as_str())
            .unwrap_or("token");
        let coin_decimals: u8 = pool
            .and_then(|p| p.coins.get(i))
            .and_then(|c| c.decimals.as_deref())
            .and_then(|d| d.parse().ok())
            .unwrap_or(18);
        let after = rpc::balance_of(addr, &wallet_addr, rpc_url).await.unwrap_or(*balances_before.get(i).unwrap_or(&0));
        let delta = after.saturating_sub(*balances_before.get(i).unwrap_or(&0));
        received.push(serde_json::json!({
            "symbol": coin_symbol,
            "address": addr,
            "amount_raw": delta.to_string(),
            "amount_human": format!("{:.6}", delta as f64 / 10f64.powi(coin_decimals as i32))
        }));
    }

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": chain_name,
            "pool_address": pool_address,
            "pool_name": pool_name,
            "lp_amount": format!("{:.6}", actual_lp_amount as f64 / 1e18),
            "lp_amount_raw": actual_lp_amount.to_string(),
            "coins_received": received,
            "tx_hash": tx_hash,
            "explorer": explorer
        })
    );
    Ok(())
}
