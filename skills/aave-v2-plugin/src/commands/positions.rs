use clap::Args;
use serde_json::{json, Value};

use crate::config::{parse_chain, supported_chains_help, ChainInfo, SUPPORTED_CHAINS};
use crate::onchainos::resolve_wallet;
use crate::rpc::{
    erc20_balance, erc20_symbol, fmt_1e18, fmt_token_amount, get_reserves_list,
    get_user_account_data, incentives_get_unclaimed_rewards, lp_get_reserve_data,
    ray_to_apr_pct,
};

#[derive(Args)]
pub struct PositionsArgs {
    /// Chain key or id (ETH / POLYGON / AVAX). Default: ETH.
    #[arg(long, default_value = "ETH")]
    pub chain: String,
    /// Wallet address (default: onchainos wallet for the chosen chain).
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let chain: &ChainInfo = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unknown --chain '{}'", args.chain),
            "INVALID_CHAIN",
            &format!("Supported: {}", supported_chains_help()),
        ),
    };

    let wallet = match args.address {
        Some(a) => a,
        None => match resolve_wallet(chain.id) {
            Ok(a) => a,
            Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
                "Run `onchainos wallet addresses` to verify login or pass --address."),
        },
    };

    let acct_fut = get_user_account_data(chain.lending_pool, &wallet, chain.rpc);
    let reserves_fut = get_reserves_list(chain.lending_pool, chain.rpc);
    let rewards_fut = async {
        if chain.incentives_controller.is_empty() { Ok(0u128) }
        else { incentives_get_unclaimed_rewards(chain.incentives_controller, &wallet, chain.rpc).await }
    };

    let (acct_res, reserves_res, rewards_res) = tokio::join!(acct_fut, reserves_fut, rewards_fut);

    // EVM-012: account data is the canonical health snapshot — RPC failure must
    // surface as an error, not a silent (0,0,0,0,0,0) fallback (which would render
    // as "totalDebt=0, HF=infinite" and mislead users into thinking they're safe
    // when their account may actually be liquidatable).
    let (total_collateral_eth, total_debt_eth, available_borrows_eth, liq_threshold, ltv, hf) =
        match acct_res {
            Ok(t) => t,
            Err(e) => return print_err(
                &format!("Failed to read account data from LendingPool on {}: {:#}", chain.key, e),
                "RPC_ERROR",
                "Public RPC may be limited; retry shortly. Account health cannot be reported \
                 without this read.",
            ),
        };
    // Rewards are non-critical (nice-to-have). Keep the 0 fallback but surface a
    // structured error indicator so callers can distinguish "no rewards accrued"
    // from "RPC failed". (EVM-012)
    let (rewards_accrued, rewards_query_error) = match rewards_res {
        Ok(v) => (v, None),
        Err(e) => (0u128, Some(format!("{:#}", e))),
    };

    let reserves: Vec<String> = match reserves_res {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("Failed to enumerate reserves on {}: {:#}", chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };

    // For each reserve: lp_get_reserve_data → get aToken/sDebt/vDebt → balanceOf user on each.
    // EVM-012: any per-balance RPC failure is now reported instead of silently
    // collapsing the reserve to (0,0,0) — which would either hide a real position
    // (when L91 filtered all-zero rows) or report "0 supply" to a user who actually
    // has a position. Reserves with at least one failed balance read are surfaced
    // in the `partial_reserves` array of the output JSON.
    let futs: Vec<_> = reserves.iter().map(|asset| {
        let chain = chain.clone();
        let wallet = wallet.clone();
        let asset = asset.clone();
        async move {
            let rd = match lp_get_reserve_data(chain.lending_pool, &asset, chain.rpc).await {
                Ok(r) => r,
                Err(e) => return Err((asset, format!("getReserveData: {:#}", e))),
            };
            let cfg = rd.decode_config();
            let dec = if cfg.decimals > 0 { cfg.decimals } else { 18 };
            let symbol_fut = erc20_symbol(&asset, chain.rpc);
            let supply_fut = erc20_balance(&rd.a_token, &wallet, chain.rpc);
            let v_debt_fut = erc20_balance(&rd.variable_debt_token, &wallet, chain.rpc);
            let s_debt_fut = erc20_balance(&rd.stable_debt_token, &wallet, chain.rpc);
            let (sym, supply, v_debt, s_debt) = tokio::join!(
                symbol_fut, supply_fut, v_debt_fut, s_debt_fut
            );
            Ok((asset, sym, dec, rd, supply, v_debt, s_debt))
        }
    }).collect();

    let results = futures::future::join_all(futs).await;

    let mut positions: Vec<Value> = Vec::new();
    let mut partial_reserves: Vec<Value> = Vec::new();
    for r in results.into_iter() {
        let (asset, sym, dec, rd, supply, v_debt, s_debt) = match r {
            Ok(t) => t,
            Err((asset, err)) => {
                partial_reserves.push(json!({ "asset": asset, "error": err }));
                continue;
            }
        };
        // Track per-balance RPC errors so we don't silently zero them out.
        let mut errs: Vec<String> = Vec::new();
        let supply_raw = supply.unwrap_or_else(|e| { errs.push(format!("supply: {:#}", e)); 0 });
        let v_debt_raw = v_debt.unwrap_or_else(|e| { errs.push(format!("variable_debt: {:#}", e)); 0 });
        let s_debt_raw = s_debt.unwrap_or_else(|e| { errs.push(format!("stable_debt: {:#}", e)); 0 });
        if !errs.is_empty() {
            partial_reserves.push(json!({
                "asset": asset,
                "symbol": sym.clone(),
                "errors": errs,
            }));
            // Don't show a reserve we couldn't fully read — partial data is worse
            // than absent data when the user is making position-management decisions.
            continue;
        }
        if supply_raw == 0 && v_debt_raw == 0 && s_debt_raw == 0 { continue; }
        positions.push(json!({
            "asset": asset,
            "symbol": sym,
            "decimals": dec,
            "supply":             fmt_token_amount(supply_raw, dec),
            "supply_raw":         supply_raw.to_string(),
            "supply_apr_pct":     format!("{:.4}", ray_to_apr_pct(rd.current_liquidity_rate_ray)),
            "variable_debt":      fmt_token_amount(v_debt_raw, dec),
            "variable_debt_raw":  v_debt_raw.to_string(),
            "variable_apr_pct":   format!("{:.4}", ray_to_apr_pct(rd.current_variable_borrow_rate_ray)),
            "stable_debt":        fmt_token_amount(s_debt_raw, dec),
            "stable_debt_raw":    s_debt_raw.to_string(),
            "stable_apr_pct":     format!("{:.4}", ray_to_apr_pct(rd.current_stable_borrow_rate_ray)),
            "a_token": rd.a_token,
        }));
    }

    let hf_display = if hf == u128::MAX || total_debt_eth == 0 {
        "infinite (no debt)".to_string()
    } else {
        fmt_1e18(hf)
    };

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "chain": chain.key,
        "chain_id": chain.id,
        "wallet": wallet,
        "lending_pool": chain.lending_pool,
        "account": {
            "total_collateral_eth_1e18":  fmt_1e18(total_collateral_eth),
            "total_debt_eth_1e18":        fmt_1e18(total_debt_eth),
            "available_borrows_eth_1e18": fmt_1e18(available_borrows_eth),
            "current_liquidation_threshold_pct": format!("{:.2}", liq_threshold as f64 / 100.0),
            "ltv_pct": format!("{:.2}", ltv as f64 / 100.0),
            "health_factor_1e18": hf_display,
            "health_factor_raw": hf.to_string(),
            "note": "ETH-equivalent values 1e18-scaled. On Polygon/Avalanche the base unit is USD (oracle-priced), labeled '_eth' for V2 ABI consistency. HF >= 1.0 healthy; < 1.0 liquidatable. ltv & liquidation_threshold are basis points / 100 = pct.",
        },
        "rewards_accrued":     fmt_token_amount(rewards_accrued, 18),
        "rewards_accrued_raw": rewards_accrued.to_string(),
        "rewards_query_error": rewards_query_error,
        "rewards_token_note": "stkAAVE on Ethereum / WMATIC on Polygon / WAVAX on Avalanche. Run claim-rewards to harvest.",
        "position_count": positions.len(),
        "positions": positions,
        "partial_reserves": partial_reserves,
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
