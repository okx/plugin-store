use clap::Args;
use serde_json::json;

use crate::config::{resolve_market_id, token_decimals, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    build_approve_max, erc20_allowance, erc20_balance, erc20_decimals, fmt_token_amount,
    get_account_wei, human_to_atomic, native_balance, pad_u256, selectors, wait_for_tx,
};

/// Repay debt on an isolated borrow position.
///
/// `--all`: clears the debt to **exactly zero** with no dust by calling
///   `BorrowPositionProxyV2.repayAllForBorrowPosition(fromAccount, borrowAccount, marketId, BalanceCheckFlag.From=1)`
/// — Dolomite's native max-sentinel that reads precise on-chain debt at execution time.
/// LEND-001 dust-free guarantee:
///   - Branch A (preferred): main account 0 has supply ≥ debt → single tx, no approve.
///   - Branch B (fallback): main short → first deposit deficit + buffer from wallet to main, then repayAll.
///   - Branch C: insufficient main + wallet → INSUFFICIENT_BALANCE error.
///
/// `--amount X`: partial repay via `depositWei(positionAccount, market, X)` — straightforward;
/// excess (X > debt) becomes supply on the position account, no dust risk.
#[derive(Args)]
pub struct RepayArgs {
    /// Token to repay (must match the borrowed token)
    #[arg(long)]
    pub token: String,
    /// Human-readable amount (use --all to repay full debt)
    #[arg(long, allow_hyphen_values = true, conflicts_with = "all")]
    pub amount: Option<String>,
    /// Repay full debt for this token via Dolomite's native exact-debt sentinel.
    /// Guarantees account ends at 0 (no dust).
    #[arg(long)]
    pub all: bool,
    /// Position account number that holds the debt (default 100 = first isolated position).
    #[arg(long, default_value = "100")]
    pub position_account_number: u128,
    /// Source account for `--all` (default 0 = main); only relevant for repay-all flow.
    #[arg(long, default_value = "0")]
    pub from_account_number: u128,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub confirm: bool,
    #[arg(long, default_value = "180")]
    pub approve_timeout_secs: u64,
}

pub async fn run(args: RepayArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    let (market_id, symbol, token_addr) = match resolve_market_id(&args.token) {
        Some(t) => t,
        None => return print_err(
            &format!("Unknown token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of USDC / USDT / WETH / DAI / WBTC / ARB / USDC.e / LINK, or pass the 0x address.",
        ),
    };
    let decimals = token_decimals(symbol)
        .or(erc20_decimals(token_addr, chain.rpc).await.ok())
        .unwrap_or(18);

    if args.amount.is_none() && !args.all {
        return print_err("Must specify --amount or --all", "INVALID_ARGUMENT",
            "Use --amount 50 (partial) or --all (clean clear via Dolomite's native sentinel).");
    }

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Read current debt for this token + position. EVM-012: RPC failure must
    // not be silently rendered as "no debt" — that would tell users repay is
    // a no-op when in fact a real obligation exists.
    let (sign, debt_value) = match get_account_wei(
        chain.dolomite_margin, &from_addr, args.position_account_number, market_id as u128, chain.rpc,
    ).await {
        Ok(t) => t,
        Err(e) => return print_err(
            &format!("Failed to read {} debt position from DolomiteMargin on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    if sign || debt_value == 0 {
        return print_err(
            &format!(
                "Account {} has no debt in {} (current balance: {} = {})",
                args.position_account_number, symbol,
                if sign { "supply" } else { "borrow" },
                fmt_token_amount(debt_value, decimals),
            ),
            "NO_DEBT",
            "Nothing to repay. Run `dolomite-plugin positions --account-number N` to see your accounts.",
        );
    }

    // Native gas pre-flight (every branch needs at least 1 tx)
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    if native < 500_000_000_000_000 {
        return print_err("Native ETH below floor", "INSUFFICIENT_GAS",
            "Top up at least 0.0005 ETH on Arbitrum.");
    }

    if args.all {
        run_repay_all(args, chain, &from_addr, market_id as u128, symbol, token_addr, decimals, debt_value).await
    } else {
        run_repay_partial(args, chain, &from_addr, market_id as u128, symbol, token_addr, decimals, debt_value).await
    }
}

/// Branch logic for `--all`: prefer Dolomite's native repayAllForBorrowPosition.
async fn run_repay_all(
    args: RepayArgs,
    chain: &crate::config::ChainInfo,
    from_addr: &str,
    market_id: u128,
    symbol: &str,
    token_addr: &str,
    decimals: u32,
    debt_value: u128,
) -> anyhow::Result<()> {
    // Read main (source) supply for this token. EVM-012: RPC failure must
    // not be silently rendered as "supply=0" — that would mis-route the
    // branch decision below (forcing a wallet-funded top-up when the user
    // actually had main-account supply available).
    let (main_sign, main_supply) = match get_account_wei(
        chain.dolomite_margin, from_addr, args.from_account_number, market_id, chain.rpc,
    ).await {
        Ok(t) => t,
        Err(e) => return print_err(
            &format!("Failed to read main-account supply from DolomiteMargin on {}: {:#}", chain.key, e),
            "RPC_ERROR",
            "Public RPC may be limited; retry shortly.",
        ),
    };
    let main_supply_atomic: u128 = if main_sign { main_supply } else { 0 };

    // Wallet balance for potential top-up. EVM-012: surface RPC errors
    // distinctly from "0 balance".
    let wallet_bal = match erc20_balance(token_addr, from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read {} wallet balance on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR", "Public RPC may be limited; retry shortly.",
        ),
    };

    // Buffer for branch B top-up: covers ~30s of interest accrual at 100% APY
    // for typical small-to-medium debts (1000 atom = $0.001 for stables, 10^15 wei
    // = $0.0035 for ETH at $3500). Cheap insurance against rate spikes.
    let buffer: u128 = 1000.max(debt_value / 10_000);  // max(1000 atom, debt × 0.01%)

    let branch = if main_supply_atomic >= debt_value {
        "A"
    } else if main_supply_atomic.saturating_add(wallet_bal) >= debt_value.saturating_add(buffer) {
        "B"
    } else {
        // Branch C — insufficient
        return print_err(
            &format!(
                "Cannot repay-all: main account 0 has {} {} supply; wallet has {} {}; together < debt {} + buffer {}.",
                fmt_token_amount(main_supply_atomic, decimals), symbol,
                fmt_token_amount(wallet_bal, decimals), symbol,
                fmt_token_amount(debt_value, decimals),
                fmt_token_amount(buffer, decimals),
            ),
            "INSUFFICIENT_BALANCE",
            "Top up your wallet with the repay token, or use --amount X for a partial repay.",
        );
    };

    // Branch B prep: how much to top up main from wallet
    let topup_atomic: u128 = if branch == "B" {
        debt_value.saturating_sub(main_supply_atomic).saturating_add(buffer)
    } else {
        0
    };

    // Build calldata for repayAllForBorrowPosition
    // selector + (fromAccount, borrowAccount, marketId, balanceCheckFlag=1 From)
    let repay_all_calldata = format!(
        "{}{}{}{}{}",
        selectors::REPAY_ALL_FOR_POSITION,
        pad_u256(args.from_account_number),
        pad_u256(args.position_account_number),
        pad_u256(market_id),
        pad_u256(1),  // BalanceCheckFlag.From — main must remain non-negative after settlement
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "repay_all",
            "chain": chain.key,
            "token": symbol,
            "market_id": market_id,
            "from_account_number": args.from_account_number,
            "position_account_number": args.position_account_number,
            "current_debt":     fmt_token_amount(debt_value, decimals),
            "current_debt_raw": debt_value.to_string(),
            "main_supply":      fmt_token_amount(main_supply_atomic, decimals),
            "main_supply_raw":  main_supply_atomic.to_string(),
            "wallet_balance":   fmt_token_amount(wallet_bal, decimals),
            "branch": branch,
            "branch_explanation": match branch {
                "A" => "Main supply ≥ debt: single repayAllForBorrowPosition tx, no approve, exact 0 dust.",
                "B" => "Main short: top-up wallet→main first, then repayAllForBorrowPosition. 2-3 txs, exact 0 dust.",
                _ => "Insufficient",
            },
            "topup_required":     fmt_token_amount(topup_atomic, decimals),
            "topup_required_raw": topup_atomic.to_string(),
            "step1_target": chain.deposit_withdrawal_proxy,
            "step2_target": chain.borrow_position_proxy,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN] Branch {}: calldata built; not signing.", branch); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm."); return Ok(()); }

    // ---- Branch B: top-up main from wallet first ----
    if branch == "B" {
        eprintln!("[repay-all] Branch B: topping up main account with {} {} from wallet…",
            fmt_token_amount(topup_atomic, decimals), symbol);

        // Approve if needed (EVM-006)
        let allowance = match erc20_allowance(token_addr, from_addr, chain.dolomite_margin, chain.rpc).await {
            Ok(v) => v,
            Err(e) => return print_err(
                &format!("Failed to read {} allowance for DolomiteMargin on {}: {:#}", symbol, chain.key, e),
                "RPC_ERROR", "Public RPC may be limited; retry shortly.",
            ),
        };
        if allowance < topup_atomic {
            let approve_data = build_approve_max(chain.dolomite_margin);
            eprintln!("[repay-all] Approving {} for DolomiteMargin…", symbol);
            let r = match wallet_contract_call(chain.id, token_addr, &approve_data, None, Some(60_000), false) {
                Ok(r) => r,
                Err(e) => return print_err(&format!("Approve failed: {:#}", e), "APPROVE_FAILED",
                    "Inspect onchainos output."),
            };
            let h = match extract_tx_hash(&r) {
                Some(h) => h,
                None => return print_err("Approve broadcast but no tx hash", "TX_HASH_MISSING",
                    "Check `onchainos wallet history`."),
            };
            eprintln!("[repay-all] Approve tx: {} — waiting…", h);
            if let Err(e) = wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await {
                return print_err(&format!("Approve confirm timeout: {:#}", e), "APPROVE_NOT_CONFIRMED",
                    "Bump --approve-timeout-secs or check explorer.");
            }
            eprintln!("[repay-all] Approve confirmed.");
        }

        // depositWei from wallet to main (account 0): selector + (toAccount, market, amount)
        let deposit_calldata = format!(
            "{}{}{}{}",
            selectors::DEPOSIT_WEI,
            pad_u256(args.from_account_number),
            pad_u256(market_id),
            pad_u256(topup_atomic),
        );
        let deposit_result = match wallet_contract_call(chain.id, chain.deposit_withdrawal_proxy, &deposit_calldata, None, Some(400_000), false) {
            Ok(r) => r,
            Err(e) => {
                let emsg = format!("{:#}", e);
                let allowance_lag = emsg.contains("transfer amount exceeds allowance")
                    || emsg.contains("exceeds allowance")
                    || emsg.contains("insufficient-allowance")
                    || emsg.contains("ERC20InsufficientAllowance");
                if allowance_lag {
                    eprintln!("[repay-all] EVM-014 allowance-lag retry, sleeping 5s…");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    wallet_contract_call(chain.id, chain.deposit_withdrawal_proxy, &deposit_calldata, None, Some(400_000), false)
                        .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
                } else {
                    return print_err(&format!("Top-up deposit failed: {:#}", emsg), "TOPUP_FAILED",
                        "Inspect onchainos output. Common: gas, RPC.");
                }
            }
        };
        let dh = match extract_tx_hash(&deposit_result) {
            Some(h) => h,
            None => return print_err("Top-up broadcast but no tx hash", "TX_HASH_MISSING",
                "Check `onchainos wallet history`."),
        };
        eprintln!("[repay-all] Top-up tx: {} — waiting…", dh);
        if let Err(e) = wait_for_tx(&dh, chain.rpc, args.approve_timeout_secs).await {
            return print_err(&format!("Top-up tx {} reverted: {:#}", dh, e), "TX_REVERTED",
                "Top-up failed on-chain. Inspect on Arbiscan.");
        }
        eprintln!("[repay-all] Top-up confirmed.");
    }

    // ---- Branch A & B common: call repayAllForBorrowPosition ----
    eprintln!("[repay-all] Calling repayAllForBorrowPosition({}, {}, {}, From=1)…",
        args.from_account_number, args.position_account_number, market_id);
    let result = match wallet_contract_call(chain.id, chain.borrow_position_proxy, &repay_all_calldata, None, Some(400_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("repayAllForBorrowPosition failed: {:#}", e),
            "REPAY_SUBMIT_FAILED",
            "Common: main account supply still insufficient (rare race), gas, RPC.",
        ),
    };
    let tx_hash = match extract_tx_hash(&result) {
        Some(h) => h,
        None => return print_err("Repay broadcast but no tx hash", "TX_HASH_MISSING",
            "Check `onchainos wallet history`."),
    };
    eprintln!("[repay-all] Repay tx: {} — waiting…", tx_hash);
    if let Err(e) = wait_for_tx(&tx_hash, chain.rpc, 180).await {
        return print_err(&format!("Tx {} reverted: {:#}", tx_hash, e), "TX_REVERTED",
            "On-chain revert. Inspect on Arbiscan.");
    }
    eprintln!("[repay-all] On-chain confirmed (status 0x1).");

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "repay_all",
        "chain": chain.key,
        "token": symbol,
        "settled_debt":     fmt_token_amount(debt_value, decimals),
        "settled_debt_raw": debt_value.to_string(),
        "branch": branch,
        "from_account_number": args.from_account_number,
        "position_account_number": args.position_account_number,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "dust_guarantee": "exact_zero (Dolomite native sentinel)",
        "tip": format!(
            "Position {} {} debt cleared to 0. Run `dolomite-plugin positions --account-number {}` to confirm. Collateral remains; recover via `withdraw --token <X> --from-account-number {} --amount <Y> --confirm`.",
            args.position_account_number, symbol,
            args.position_account_number, args.position_account_number,
        ),
    }))?);
    Ok(())
}

/// Partial repay: `depositWei(positionAccount, market, amount)` from wallet.
async fn run_repay_partial(
    args: RepayArgs,
    chain: &crate::config::ChainInfo,
    from_addr: &str,
    market_id: u128,
    symbol: &str,
    token_addr: &str,
    decimals: u32,
    debt_value: u128,
) -> anyhow::Result<()> {
    let amount_raw = match human_to_atomic(args.amount.as_ref().unwrap(), decimals) {
        Ok(v) => v.min(debt_value),
        Err(e) => return print_err(&format!("Invalid --amount: {}", e), "INVALID_ARGUMENT",
            "Pass a positive number or --all."),
    };

    // Pre-flight: wallet must have enough of the repay token
    let bal = match erc20_balance(token_addr, from_addr, chain.rpc).await {
        Ok(v) => v,
        Err(e) => return print_err(
            &format!("Failed to read {} wallet balance on {}: {:#}", symbol, chain.key, e),
            "RPC_ERROR", "Public RPC may be limited; retry shortly.",
        ),
    };
    if bal < amount_raw {
        return print_err(
            &format!(
                "Insufficient {} in wallet to repay: need {} (raw {}), have {} (raw {}).",
                symbol, fmt_token_amount(amount_raw, decimals), amount_raw,
                fmt_token_amount(bal, decimals), bal,
            ),
            "INSUFFICIENT_BALANCE",
            "Top up the repay token, or reduce --amount.",
        );
    }

    let calldata = format!(
        "{}{}{}{}",
        selectors::DEPOSIT_WEI,
        pad_u256(args.position_account_number),
        pad_u256(market_id),
        pad_u256(amount_raw),
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "repay_partial",
            "chain": chain.key,
            "token": symbol,
            "market_id": market_id,
            "position_account_number": args.position_account_number,
            "current_debt":     fmt_token_amount(debt_value, decimals),
            "current_debt_raw": debt_value.to_string(),
            "amount":     fmt_token_amount(amount_raw, decimals),
            "amount_raw": amount_raw.to_string(),
            "wallet_balance": fmt_token_amount(bal, decimals),
            "spender": chain.dolomite_margin,
            "call_target": chain.deposit_withdrawal_proxy,
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN]"); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm."); return Ok(()); }

    // Approve if needed
    let allowance = erc20_allowance(token_addr, from_addr, chain.dolomite_margin, chain.rpc).await.unwrap_or(0);
    if allowance < amount_raw {
        let approve_data = build_approve_max(chain.dolomite_margin);
        eprintln!("[repay] Approving {} for DolomiteMargin…", symbol);
        let r = wallet_contract_call(chain.id, token_addr, &approve_data, None, Some(60_000), false)
            .map_err(|e| anyhow::anyhow!("approve failed: {:#}", e))?;
        let h = extract_tx_hash(&r).ok_or_else(|| anyhow::anyhow!("approve tx hash missing"))?;
        eprintln!("[repay] Approve tx: {} — waiting…", h);
        wait_for_tx(&h, chain.rpc, args.approve_timeout_secs).await
            .map_err(|e| anyhow::anyhow!("approve confirm timeout: {:#}", e))?;
        eprintln!("[repay] Approve confirmed.");
    }

    let result = match wallet_contract_call(chain.id, chain.deposit_withdrawal_proxy, &calldata, None, Some(400_000), false) {
        Ok(r) => r,
        Err(e) => {
            let emsg = format!("{:#}", e);
            let allowance_lag = emsg.contains("transfer amount exceeds allowance")
                || emsg.contains("exceeds allowance")
                || emsg.contains("insufficient-allowance")
                || emsg.contains("ERC20InsufficientAllowance");
            if allowance_lag {
                eprintln!("[repay] EVM-014 allowance-lag retry, sleeping 5s…");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                wallet_contract_call(chain.id, chain.deposit_withdrawal_proxy, &calldata, None, Some(400_000), false)
                    .map_err(|e2| anyhow::anyhow!("retry failed: {:#}", e2))?
            } else {
                return print_err(&format!("Repay submission failed: {:#}", emsg), "REPAY_SUBMIT_FAILED",
                    "Inspect onchainos output. Common: gas, RPC.");
            }
        }
    };
    let tx_hash = match extract_tx_hash(&result) {
        Some(h) => h,
        None => return print_err("Repay broadcast but no tx hash", "TX_HASH_MISSING",
            "Check `onchainos wallet history`."),
    };
    eprintln!("[repay] Submit tx: {} — waiting…", tx_hash);
    if let Err(e) = wait_for_tx(&tx_hash, chain.rpc, 180).await {
        return print_err(&format!("Tx {} reverted: {:#}", tx_hash, e), "TX_REVERTED",
            "On-chain revert. Inspect on Arbiscan.");
    }
    eprintln!("[repay] On-chain confirmed.");

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "repay_partial",
        "chain": chain.key,
        "token": symbol,
        "amount":     fmt_token_amount(amount_raw, decimals),
        "amount_raw": amount_raw.to_string(),
        "position_account_number": args.position_account_number,
        "tx_hash": tx_hash,
        "on_chain_status": "0x1",
        "tip": "Run `dolomite-plugin positions --account-number N` to verify remaining debt.",
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
