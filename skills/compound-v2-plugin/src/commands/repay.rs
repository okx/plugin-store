use clap::Args;
use serde_json::json;

use crate::config::{resolve_market, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    borrow_balance_current, build_approve_max, erc20_allowance, erc20_balance, fmt_token_amount,
    human_to_atomic, native_balance, pad_u256, pad_u256_max, selectors, wait_for_tx,
};

/// Repay debt on a Compound V2 cToken market.
///
/// `--all`: passes `uint256.max` (`0xff…ff`) as the amount. Compound V2's `repayBorrow`
/// auto-caps to `min(amount, currentBorrowBalance)` at execution time → settles to
/// **exactly zero (no dust)** — addresses LEND-001. Same mechanism as Aave V3 max-sentinel.
///
/// `--amount X`: partial repay. Excess is auto-clamped at borrowBalanceCurrent (so passing
/// X > debt has the same effect as `--all`).
#[derive(Args)]
pub struct RepayArgs {
    /// Token to repay (DAI / USDC / USDT / ETH / WBTC / COMP) or 0x address
    #[arg(long)]
    pub token: String,
    /// Underlying amount (use --all for full debt clearance via uint256.max sentinel)
    #[arg(long, allow_hyphen_values = true, conflicts_with = "all")]
    pub amount: Option<String>,
    /// Repay full debt — uses uint256.max sentinel (LEND-001 dust-free guarantee)
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: RepayArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let info = match resolve_market(&args.token) {
        Some(i) => i,
        None => return print_err(
            &format!("Unknown token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of DAI / USDC / USDT / ETH / WBTC / COMP.",
        ),
    };

    if args.amount.is_none() && !args.all {
        return print_err("Must specify --amount or --all", "INVALID_ARGUMENT",
            "Use --amount 50 (partial) or --all (uint256.max sentinel — dust-free).");
    }

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Read current debt
    let debt_raw = match borrow_balance_current(info.ctoken, &from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(&format!("RPC: {}", e), "RPC_ERROR",
            "Public Ethereum RPC may be limited; retry shortly."),
    };
    if debt_raw == 0 {
        return print_err(
            &format!("No {} debt on Compound V2 (current borrow: 0).", info.underlying_symbol),
            "NO_DEBT",
            "Nothing to repay. Run `compound-v2-plugin positions` to see all balances.",
        );
    }

    // Determine amount + calldata strategy
    // For --all, we send uint256.max — Compound's repayBorrow caps internally.
    // We still need to pre-flight wallet balance for the wallet → cToken transferFrom.
    let (amount_raw_for_check, amount_calldata_hex): (u128, String) = if args.all {
        // For wallet balance check, use current debt as estimate (real settle could be slightly higher).
        // We add a small headroom — but since Compound caps, sending more than debt is harmless except
        // we need wallet ≥ actual debt at execution time. Use debt + 0.1% buffer, or supplied amount
        // floor of debt × 1.001 + 1 atom.
        let buffer = (debt_raw / 1000).max(1);
        (debt_raw.saturating_add(buffer), pad_u256_max())
    } else {
        let user_atomic = match human_to_atomic(args.amount.as_ref().unwrap(), info.underlying_decimals) {
            Ok(v) => v,
            Err(e) => return print_err(&format!("Invalid --amount: {}", e),
                "INVALID_ARGUMENT", "Pass a positive number or --all"),
        };
        // Cap at current debt (saves wallet funds; Compound would do this anyway but explicit is cleaner)
        let capped = user_atomic.min(debt_raw);
        (capped, pad_u256(capped))
    };

    // Pre-flight: wallet balance (and ETH gas). EVM-012: surface RPC failures
    // distinctly from "user has 0 balance" — silent unwrap_or(0) here used to
    // misreport INSUFFICIENT_BALANCE on every public-RPC blip.
    let wallet_bal = if info.is_native {
        match native_balance(&from_addr, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read native balance on {}: {:#}", chain.key, e),
                "RPC_ERROR", "Public RPC may be limited; retry shortly.",
            ),
        }
    } else {
        match erc20_balance(info.underlying, &from_addr, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read {} wallet balance on {}: {:#}", info.underlying_symbol, chain.key, e),
                "RPC_ERROR", "Public RPC may be limited; retry shortly.",
            ),
        }
    };
    if wallet_bal < amount_raw_for_check {
        return print_err(
            &format!(
                "Insufficient {} in wallet: need ~{} (raw {}), have {} (raw {}).",
                info.underlying_symbol,
                fmt_token_amount(amount_raw_for_check, info.underlying_decimals), amount_raw_for_check,
                fmt_token_amount(wallet_bal, info.underlying_decimals), wallet_bal,
            ),
            "INSUFFICIENT_BALANCE",
            if args.all {
                "Top up the repay token to at least your current debt + 0.1% buffer (interest accrues per block; sentinel uint256.max needs wallet ≥ exact debt at execution)."
            } else {
                "Reduce --amount, or top up the repay token."
            },
        );
    }
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    let gas_floor: u128 = if info.is_native { amount_raw_for_check + 5_000_000_000_000_000 } else { 5_000_000_000_000_000 };
    if native < gas_floor {
        return print_err("Native ETH below floor", "INSUFFICIENT_GAS",
            "Top up at least 0.005 ETH on mainnet.");
    }

    // For ERC-20 underlyings: approve cToken to pull underlying
    if !info.is_native {
        let allowance = match erc20_allowance(info.underlying, &from_addr, info.ctoken, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read {} allowance for cToken on {}: {:#}", info.underlying_symbol, chain.key, e),
                "RPC_ERROR", "Public RPC may be limited; retry shortly.",
            ),
        };
        if allowance < amount_raw_for_check && !args.dry_run && args.confirm {
            // (Skip approve when dry_run / preview — we still emit JSON below)
        }
    }

    // Calldata: repayBorrow(amount)
    let calldata = format!("{}{}", selectors::REPAY_BORROW, amount_calldata_hex);

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "repay",
            "chain": chain.key,
            "ctoken": info.ctoken,
            "underlying_symbol": info.underlying_symbol,
            "is_native": info.is_native,
            "current_debt":     fmt_token_amount(debt_raw, info.underlying_decimals),
            "current_debt_raw": debt_raw.to_string(),
            "amount_to_send":   if args.all { "uint256.max (sentinel)".to_string() }
                                else { fmt_token_amount(amount_raw_for_check, info.underlying_decimals) },
            "is_repay_all": args.all,
            "wallet_balance": fmt_token_amount(wallet_bal, info.underlying_decimals),
            "dust_guarantee": if args.all { "exact_zero (Compound V2 native uint256.max sentinel)" }
                              else { "amount-based (no dust if amount ≥ debt)" },
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN] Calldata built; not signing."); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    // Approve (ERC-20 only, max approve so user doesn't have to re-approve next time)
    if !info.is_native {
        let allowance = match erc20_allowance(info.underlying, &from_addr, info.ctoken, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read {} allowance for cToken on {}: {:#}", info.underlying_symbol, chain.key, e),
                "RPC_ERROR", "Public RPC may be limited; retry shortly.",
            ),
        };
        if allowance < amount_raw_for_check {
            let approve_data = build_approve_max(info.ctoken);
            eprintln!("[repay] Approving {} for cToken contract…", info.underlying_symbol);
            let r = match wallet_contract_call(chain.id, info.underlying, &approve_data, None, Some(80_000), false) {
                Ok(r) => r,
                Err(e) => return print_err(&format!("Approve failed: {:#}", e),
                    "APPROVE_FAILED", "Inspect onchainos output."),
            };
            let h = match extract_tx_hash(&r) {
                Some(h) => h,
                None => return print_err("Approve broadcast but no tx hash",
                    "TX_HASH_MISSING", "Check `onchainos wallet history`."),
            };
            eprintln!("[repay] Approve tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Approve confirm timeout: {:#}", e),
                    "APPROVE_NOT_CONFIRMED", "Bump --approve-timeout-secs.");
            }
            eprintln!("[repay] Approve confirmed.");
        }
    }

    // Submit repayBorrow (EVM-014 retry on allowance lag, EVM-015 explicit gas)
    let value_wei = if info.is_native { Some(amount_raw_for_check) } else { None };
    let result = match wallet_contract_call(chain.id, info.ctoken, &calldata, value_wei, Some(280_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            let allowance_lag = !info.is_native && (
                emsg.contains("transfer amount exceeds allowance")
                || emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance")
            );
            if allowance_lag {
                eprintln!("[repay] EVM-014 allowance-lag retry, sleeping 5s…");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, info.ctoken, &calldata, value_wei, Some(280_000), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(&format!("repayBorrow failed: {:#}", emsg),
                    "REPAY_SUBMIT_FAILED", "Inspect onchainos output. Common: gas, RPC.");
            }
        }
    };
    let tx_hash = extract_tx_hash(&result);

    match tx_hash.as_ref() {
        Some(h) => {
            eprintln!("[repay] Submit tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Tx {} reverted: {:#}", h, e),
                    "TX_REVERTED", "On-chain revert. Inspect on Etherscan.");
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
        "underlying_symbol": info.underlying_symbol,
        "is_repay_all": args.all,
        "settled_debt":     fmt_token_amount(debt_raw, info.underlying_decimals),
        "settled_debt_raw": debt_raw.to_string(),
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "dust_guarantee": if args.all { "exact_zero (uint256.max sentinel)" } else { "amount-based" },
        "tip": "Run `compound-v2-plugin positions` to confirm new debt.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
