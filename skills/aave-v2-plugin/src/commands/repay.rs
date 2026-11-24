use clap::Args;
use serde_json::json;

use crate::config::{parse_chain, supported_chains_help, ChainInfo, RateMode, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, erc20_decimals, erc20_symbol,
    fmt_token_amount, get_reserves_list, human_to_atomic, lp_get_reserve_data,
    native_balance, pad_address, pad_u256, pad_u256_max, selectors, wait_for_tx,
};

/// Repay debt via LendingPool.repay(asset, amount, rateMode, onBehalfOf).
///
/// `--all`: passes `uint256.max` as amount. Aave V2's repay() auto-caps to
/// `min(amount, currentDebt)` at execution time -> exactly zero dust on the targeted
/// rate mode. Same mechanism as Aave V3 / Compound V2 max-sentinel. Addresses LEND-001.
///
/// `--rate-mode 1` repays stable debt; `--rate-mode 2` repays variable debt. If user
/// has both, repay each separately (or specify the larger first to clear it).
///
/// All operations require explicit `--confirm`. v0.1.0 ERC-20 only.
#[derive(Args)]
pub struct RepayArgs {
    /// Chain key or id (ETH / POLYGON / AVAX).
    #[arg(long, default_value = "ETH")]
    pub chain: String,
    /// Token to repay (case-insensitive symbol or 0x address)
    #[arg(long)]
    pub token: String,
    /// Underlying amount (use --all for full debt clearance via uint256.max sentinel)
    #[arg(long, allow_hyphen_values = true, conflicts_with = "all")]
    pub amount: Option<String>,
    /// Repay full debt for this rate-mode (uint256.max sentinel - LEND-001 dust-free)
    #[arg(long)]
    pub all: bool,
    /// Interest rate mode of the debt to repay: 1=stable, 2=variable.
    #[arg(long)]
    pub rate_mode: u8,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: RepayArgs) -> anyhow::Result<()> {
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
            "INVALID_ARGUMENT", "Pass --rate-mode 2 for variable or 1 for stable.",
        ),
    };

    if args.amount.is_none() && !args.all {
        return print_err("Must specify --amount or --all", "INVALID_ARGUMENT",
            "Use --amount 50 (partial) or --all (uint256.max sentinel - dust-free).");
    }

    let upper = args.token.to_uppercase();
    if upper == chain.native_symbol {
        return print_err(
            &format!("Native {} repay deferred to v0.2.0. Repay the wrapped W{} debt instead.",
                chain.native_symbol, chain.native_symbol),
            "NATIVE_NOT_SUPPORTED_V01",
            &format!("Use --token W{} (wrapped).", chain.native_symbol),
        );
    }

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

    // Read current debt: get sDebt/vDebt addresses via LendingPool, then balanceOf(user)
    let rd = match lp_get_reserve_data(chain.lending_pool, &asset_addr, chain.rpc).await {
        Ok(r) => r,
        Err(e) => return print_err(&format!("LendingPool.getReserveData: {}", e), "RPC_ERROR",
            "Public RPC may be limited; retry shortly."),
    };
    let debt_token = if mode == RateMode::Stable { &rd.stable_debt_token } else { &rd.variable_debt_token };
    // EVM-012: distinguish RPC failure from "user has no debt". Silent unwrap_or(0)
    // here used to tell users "nothing to repay" when the public RPC actually just
    // failed — leading them to think they had no obligation when they did.
    let debt_raw = match erc20_balance(debt_token, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read {}-debt balance for {} on {}: {:#}",
                if args.rate_mode == 1 { "stable" } else { "variable" }, symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if debt_raw == 0 {
        return print_err(
            &format!("No {} {}-rate debt on Aave V2 {} (current: 0).",
                symbol, if args.rate_mode == 1 { "stable" } else { "variable" }, chain.key),
            "NO_DEBT",
            "Nothing to repay. Try --rate-mode 2 (or 1) for the other mode, or run `positions`.",
        );
    }

    // Determine repay amount
    let (amount_raw_for_check, amount_calldata): (u128, String) = if args.all {
        // For wallet balance check: actual debt + 0.1% buffer; sentinel handles capping.
        let buffer = (debt_raw / 1000).max(1);
        (debt_raw.saturating_add(buffer), pad_u256_max())
    } else {
        let user_atomic = match human_to_atomic(args.amount.as_ref().unwrap(), decimals) {
            Ok(v) => v,
            Err(e) => return print_err(&format!("Invalid --amount: {}", e),
                "INVALID_ARGUMENT", "Pass a positive number or --all"),
        };
        let capped = user_atomic.min(debt_raw);
        (capped, pad_u256(capped))
    };

    // Pre-flight: wallet balance. EVM-012: surface RPC failures rather than
    // reporting bal=0 (which would always trigger INSUFFICIENT_BALANCE).
    let bal = match erc20_balance(&asset_addr, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read {} wallet balance on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if bal < amount_raw_for_check {
        return print_err(
            &format!("Insufficient {} in wallet: need ~{} (raw {}), have {} (raw {}).",
                symbol, fmt_token_amount(amount_raw_for_check, decimals), amount_raw_for_check,
                fmt_token_amount(bal, decimals), bal),
            "INSUFFICIENT_BALANCE",
            if args.all {
                "Top up at least your current debt + 0.1% buffer (interest accrues per second; sentinel uint256.max needs wallet >= exact debt at execution)."
            } else {
                "Reduce --amount or top up the repay token."
            },
        );
    }

    // Pre-flight: native gas
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < chain.gas_floor_wei {
        return print_err(
            &format!("Native {} insufficient on {}", chain.native_symbol, chain.key),
            "INSUFFICIENT_GAS", "Top up native gas.",
        );
    }

    // Build calldata: repay(asset, amount, rateMode, onBehalfOf)
    let calldata = format!("{}{}{}{}{}",
        selectors::REPAY,
        pad_address(&asset_addr),
        amount_calldata,
        pad_u256(mode.as_u128()),
        pad_address(&from_addr),
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "repay",
            "chain": chain.key,
            "asset": asset_addr,
            "symbol": symbol,
            "rate_mode": args.rate_mode,
            "rate_mode_label": if args.rate_mode == 1 { "stable" } else { "variable" },
            "current_debt":     fmt_token_amount(debt_raw, decimals),
            "current_debt_raw": debt_raw.to_string(),
            "amount_to_send":   if args.all { "uint256.max (sentinel - settles to exact debt)".to_string() }
                                else { fmt_token_amount(amount_raw_for_check, decimals) },
            "is_repay_all": args.all,
            "wallet_balance": fmt_token_amount(bal, decimals),
            "dust_guarantee": if args.all { "exact_zero (Aave V2 native uint256.max sentinel)" }
                              else { "amount-based (no dust if amount >= debt)" },
            "call_target": chain.lending_pool,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    // Approve LendingPool (max approve). EVM-012: prefer surfacing the RPC
    // failure over silently treating allowance as 0 (which would force a
    // redundant approve every time the RPC blips, wasting gas).
    let allowance = match erc20_allowance(&asset_addr, &from_addr, chain.lending_pool, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read {} allowance for LendingPool on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if allowance < amount_raw_for_check {
        let approve_data = build_approve_max(chain.lending_pool);
        eprintln!("[repay] Approving {} for LendingPool...", symbol);
        let r = match wallet_contract_call(chain.id, &asset_addr, &approve_data, None, Some(80_000), false) {
            Ok(r) => r,
            Err(e) => return print_err(&format!("Approve failed: {:#}", e),
                "APPROVE_FAILED", "Inspect onchainos output."),
        };
        let h = match extract_tx_hash(&r) {
            Some(h) => h,
            None => return print_err("Approve broadcast but no tx hash",
                "TX_HASH_MISSING", "Check `onchainos wallet history`."),
        };
        eprintln!("[repay] Approve tx: {} - waiting...", h);
        if let Err(e) = wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await {
            return print_err(&format!("Approve confirm timeout: {:#}", e),
                "APPROVE_NOT_CONFIRMED", "Bump --approve-timeout-secs.");
        }
        eprintln!("[repay] Approve confirmed.");
    }

    // Submit repay (EVM-014 retry on allowance lag, EVM-015 explicit gas)
    let result = match wallet_contract_call(chain.id, chain.lending_pool, &calldata, None, Some(350_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            let allowance_lag = emsg.contains("transfer amount exceeds allowance")
                || emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance");
            if allowance_lag {
                eprintln!("[repay] EVM-014 allowance-lag retry, sleeping 5s...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, chain.lending_pool, &calldata, None, Some(350_000), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(&format!("repay failed: {:#}", emsg),
                    "REPAY_SUBMIT_FAILED", "Inspect onchainos output. Common: gas, RPC.");
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[repay] Submit tx: {} - waiting...", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on the block explorer.");
            }
            eprintln!("[repay] On-chain confirmed (status 0x1).");
        }
        None => return print_err("Repay broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    }

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "repay",
        "chain": chain.key,
        "asset": asset_addr,
        "symbol": symbol,
        "rate_mode": args.rate_mode,
        "is_repay_all": args.all,
        "settled_debt":     fmt_token_amount(debt_raw, decimals),
        "settled_debt_raw": debt_raw.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "dust_guarantee": if args.all { "exact_zero (uint256.max sentinel)" } else { "amount-based" },
        "tip": "Run `aave-v2-plugin positions --chain X` to confirm new debt.",
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
