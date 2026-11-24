use clap::Args;
use serde_json::{json, Value};

use crate::config::{parse_chain, supported_chains_help, ChainInfo, SUPPORTED_CHAINS};
use crate::rpc::{
    bps_to_pct, erc20_balance, erc20_symbol, erc20_total_supply, fmt_token_amount,
    get_reserves_list, lp_get_reserve_data, ray_to_apr_pct,
};

#[derive(Args)]
pub struct MarketsArgs {
    /// Chain key or id (ETH / POLYGON / AVAX). Default: ETH.
    #[arg(long, default_value = "ETH")]
    pub chain: String,
    /// Limit number of reserves shown (default: all). Useful on Ethereum (37 reserves).
    #[arg(long)]
    pub limit: Option<usize>,
}

pub async fn run(args: MarketsArgs) -> anyhow::Result<()> {
    let chain: &ChainInfo = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unknown --chain '{}'", args.chain),
            "INVALID_CHAIN",
            &format!("Supported: {}", supported_chains_help()),
        ),
    };

    let reserves = match get_reserves_list(chain.lending_pool, chain.rpc).await {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("Failed to enumerate reserves on {}: {:#}", chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };

    // Per-reserve parallel: LendingPool reserve data + symbol + cash/totals
    let futs: Vec<_> = reserves.iter().map(|asset| {
        let chain = chain.clone();
        let asset = asset.clone();
        async move {
            let rd = match lp_get_reserve_data(chain.lending_pool, &asset, chain.rpc).await {
                Ok(r) => r,
                Err(_) => return (asset, "?".to_string(), None),
            };
            let cfg = rd.decode_config();
            let dec = if cfg.decimals > 0 { cfg.decimals } else { 18 };
            let symbol_fut = erc20_symbol(&asset, chain.rpc);
            let avail_fut = erc20_balance(&asset, &rd.a_token, chain.rpc);
            let total_a_fut = erc20_total_supply(&rd.a_token, chain.rpc);
            let total_s_fut = erc20_total_supply(&rd.stable_debt_token, chain.rpc);
            let total_v_fut = erc20_total_supply(&rd.variable_debt_token, chain.rpc);
            let (sym, avail, total_a, total_s, total_v) = tokio::join!(
                symbol_fut, avail_fut, total_a_fut, total_s_fut, total_v_fut
            );
            // EVM-012: track per-balance errors so the output JSON can flag
            // partial data instead of silently rendering 0 (which would
            // misreport an empty market or break utilization math via /0).
            let mut errs: Vec<String> = Vec::new();
            let avail = avail.unwrap_or_else(|e| { errs.push(format!("available_liquidity: {:#}", e)); 0 });
            let total_a = total_a.unwrap_or_else(|e| { errs.push(format!("total_a_supply: {:#}", e)); 0 });
            let total_s = total_s.unwrap_or_else(|e| { errs.push(format!("total_s_debt: {:#}", e)); 0 });
            let total_v = total_v.unwrap_or_else(|e| { errs.push(format!("total_v_debt: {:#}", e)); 0 });
            (asset, sym, Some((rd, cfg, dec, avail, total_a, total_s, total_v, errs)))
        }
    }).collect();

    let results = futures::future::join_all(futs).await;

    let entries: Vec<Value> = results.into_iter().map(|(asset, sym, payload)| {
        match payload {
            None => json!({"asset": asset, "symbol": sym, "error": "RPC failed (getReserveData)"}),
            Some((rd, cfg, dec, avail, total_a, total_s, total_v, errs)) => {
                let total_debt = total_s + total_v;
                let utilization_pct = if total_a > 0 {
                    Some(format!("{:.2}", (total_debt as f64 / total_a as f64) * 100.0))
                } else { None };
                json!({
                    "asset": asset,
                    "symbol": sym,
                    "decimals": dec,
                    "a_token": rd.a_token,
                    "s_debt_token": rd.stable_debt_token,
                    "v_debt_token": rd.variable_debt_token,
                    "supply_apr_pct":          format!("{:.4}", ray_to_apr_pct(rd.current_liquidity_rate_ray)),
                    "variable_borrow_apr_pct": format!("{:.4}", ray_to_apr_pct(rd.current_variable_borrow_rate_ray)),
                    "stable_borrow_apr_pct":   format!("{:.4}", ray_to_apr_pct(rd.current_stable_borrow_rate_ray)),
                    "available_liquidity":     fmt_token_amount(avail, dec),
                    "available_liquidity_raw": avail.to_string(),
                    "total_stable_debt":       fmt_token_amount(total_s, dec),
                    "total_variable_debt":     fmt_token_amount(total_v, dec),
                    "total_supply_underlying": fmt_token_amount(total_a, dec),
                    "utilization_pct": utilization_pct,
                    "liquidity_index_ray": rd.liquidity_index_ray.to_string(),
                    "config": {
                        "ltv_pct":                   bps_to_pct(cfg.ltv_bps as u128),
                        "liquidation_threshold_pct": bps_to_pct(cfg.liq_threshold_bps as u128),
                        "liquidation_bonus_pct":     bps_to_pct(cfg.liq_bonus_bps as u128),
                        "reserve_factor_pct":        bps_to_pct(cfg.reserve_factor_bps as u128),
                        "borrowing_enabled":         cfg.borrowing_enabled,
                        "stable_borrow_rate_enabled": cfg.stable_rate_enabled,
                        "is_active": cfg.is_active,
                        "is_frozen": cfg.is_frozen,
                    },
                    "partial_data_errors": if errs.is_empty() { None } else { Some(errs) },
                })
            }
        }
    }).collect();

    let limit = args.limit.unwrap_or(entries.len()).min(entries.len());
    let displayed = &entries[..limit];

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "chain": chain.key,
        "chain_id": chain.id,
        "lending_pool": chain.lending_pool,
        "weth_gateway": chain.weth_gateway,
        "incentives_controller": chain.incentives_controller,
        "reserves_total": entries.len(),
        "reserves_displayed": displayed.len(),
        "reserves": displayed,
        "note": "Reserves enumerated at runtime via LendingPool.getReservesList(); rates/config decoded from LendingPool.getReserveData (the canonical Aave V2 PDP at 0x057835...3B16B36 has no code on Ethereum mainnet, so we source data from LendingPool directly + ERC-20 calls on aToken/sDebt/vDebt). Rates are 1e27 ray-scaled annual; config fields in basis points (10000 = 100%).",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
