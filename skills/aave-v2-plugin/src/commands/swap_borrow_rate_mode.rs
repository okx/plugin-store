use clap::Args;
use serde_json::json;

use crate::config::{parse_chain, supported_chains_help, ChainInfo, RateMode, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    erc20_balance, erc20_decimals, erc20_symbol, fmt_token_amount, get_reserves_list,
    lp_get_reserve_data, native_balance, pad_address, pad_u256, selectors, wait_for_tx,
};

/// Swap an existing borrow's interest rate mode between stable (1) and variable (2).
/// Calls LendingPool.swapBorrowRateMode(asset, rateMode). The `rateMode` argument is
/// the user's CURRENT mode (the one being swapped FROM).
///
/// Use case: variable rates have spiked and you want to lock in a stable rate (or
/// vice versa - stable rate has been rebalanced upwards and variable looks cheaper).
///
/// V2-only feature; V3 removed stable mode entirely.
///
/// All operations require explicit `--confirm`.
#[derive(Args)]
pub struct SwapBorrowRateModeArgs {
    /// Chain key or id (ETH / POLYGON / AVAX).
    #[arg(long, default_value = "ETH")]
    pub chain: String,
    /// Token whose borrow rate mode to swap (case-insensitive symbol or 0x address).
    #[arg(long)]
    pub token: String,
    /// CURRENT rate mode of the borrow (the one you want to swap FROM).
    /// 1=stable -> swap to variable; 2=variable -> swap to stable.
    #[arg(long)]
    pub rate_mode: u8,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

pub async fn run(args: SwapBorrowRateModeArgs) -> anyhow::Result<()> {
    let chain: &ChainInfo = match parse_chain(&args.chain) {
        Some(c) => c,
        None => return print_err(
            &format!("Unknown --chain '{}'", args.chain),
            "INVALID_CHAIN",
            &format!("Supported: {}", supported_chains_help()),
        ),
    };

    let mode = match RateMode::from_u8(args.rate_mode) {
        Some(m) => m,
        None => return print_err(
            &format!("Invalid --rate-mode '{}': must be 1 or 2", args.rate_mode),
            "INVALID_ARGUMENT",
            "Pass --rate-mode N where N is your CURRENT mode (1=stable, 2=variable).",
        ),
    };

    let (asset_addr, symbol, decimals) = if args.token.starts_with("0x") && args.token.len() == 42 {
        let dec = erc20_decimals(&args.token, chain.rpc).await
            .map_err(|e| anyhow::anyhow!("erc20 decimals: {}", e))?;
        let sym = erc20_symbol(&args.token, chain.rpc).await;
        (args.token.to_lowercase(), sym, dec)
    } else {
        match resolve_symbol(&args.token, chain).await {
            Some(t) => t,
            None => return print_err(
                &format!("Token '{}' not found among Aave V2 reserves on {}", args.token, chain.key),
                "TOKEN_NOT_FOUND",
                "Run `aave-v2-plugin markets --chain X` to see all listed reserves.",
            ),
        }
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Pre-flight: verify user has debt in the specified rate mode
    let rd = match lp_get_reserve_data(chain.lending_pool, &asset_addr, chain.rpc).await {
        Ok(r) => r,
        Err(e) => return print_err(&format!("LendingPool.getReserveData: {}", e), "RPC_ERROR",
            "Public RPC may be limited; retry shortly."),
    };
    let debt_token = if mode == RateMode::Stable { &rd.stable_debt_token } else { &rd.variable_debt_token };
    // EVM-012: surface RPC failures distinctly from "user has no debt" — a
    // silent unwrap_or(0) here would tell users to "swap to a mode with actual
    // debt" when the real issue is the public RPC being unavailable.
    let current_debt_in_mode = match erc20_balance(debt_token, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read {}-debt balance for {} on {}: {:#}",
                if args.rate_mode == 1 { "stable" } else { "variable" }, symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if current_debt_in_mode == 0 {
        return print_err(
            &format!("No {} {}-rate debt on Aave V2 {} - nothing to swap.",
                symbol, if args.rate_mode == 1 { "stable" } else { "variable" }, chain.key),
            "NO_DEBT_IN_MODE",
            "Pass the rate-mode that has actual debt. Run `positions` to see your debt breakdown.",
        );
    }

    // Pre-flight: gas
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < chain.gas_floor_wei {
        return print_err(
            &format!("Native {} insufficient on {}", chain.native_symbol, chain.key),
            "INSUFFICIENT_GAS", "Top up native gas.",
        );
    }

    // Build calldata: swapBorrowRateMode(asset, rateMode)
    let calldata = format!("{}{}{}",
        selectors::SWAP_BORROW_RATE_MODE,
        pad_address(&asset_addr),
        pad_u256(mode.as_u128()),
    );

    let target_mode = if args.rate_mode == 1 { 2 } else { 1 };
    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "swap_borrow_rate_mode",
            "chain": chain.key,
            "asset": asset_addr,
            "symbol": symbol,
            "from_rate_mode": args.rate_mode,
            "from_rate_mode_label": if args.rate_mode == 1 { "stable" } else { "variable" },
            "to_rate_mode": target_mode,
            "to_rate_mode_label": if target_mode == 1 { "stable" } else { "variable" },
            "current_debt_in_mode":     fmt_token_amount(current_debt_in_mode, decimals),
            "current_debt_in_mode_raw": current_debt_in_mode.to_string(),
            "call_target": chain.lending_pool,
            "warning": "Swap fails if target mode is disabled for this reserve (some reserves don't allow stable). Check `markets` for stable_borrow_rate_enabled flag.",
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    let result = match wallet_contract_call(chain.id, chain.lending_pool, &calldata, None, Some(250_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(&format!("swapBorrowRateMode failed: {:#}", e),
            "SWAP_FAILED",
            "Common: target mode disabled (stable_borrow_rate_enabled=false), gas, RPC."),
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[swap-rate-mode] Submit tx: {} - waiting...", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on the block explorer.");
            }
            eprintln!("[swap-rate-mode] On-chain confirmed.");
        }
        None => return print_err("swap broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "swap_borrow_rate_mode",
        "chain": chain.key,
        "asset": asset_addr,
        "symbol": symbol,
        "from_rate_mode": args.rate_mode,
        "to_rate_mode": target_mode,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `aave-v2-plugin positions --chain X` to verify debt is now in the new mode.",
    }))?);
    Ok(())
}

async fn resolve_symbol(token: &str, chain: &ChainInfo) -> Option<(String, String, u32)> {
    let reserves = get_reserves_list(chain.lending_pool, chain.rpc).await.ok()?;
    let upper = token.to_uppercase();
    for asset in reserves {
        let sym = erc20_symbol(&asset, chain.rpc).await;
        if sym.to_uppercase() == upper {
            let dec = erc20_decimals(&asset, chain.rpc).await.unwrap_or(18);
            return Some((asset, sym, dec));
        }
    }
    None
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
