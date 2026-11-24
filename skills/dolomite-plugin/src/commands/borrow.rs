use clap::Args;
use serde_json::json;

use crate::config::{resolve_market_id, token_decimals, SUPPORTED_CHAINS};
use crate::onchainos::{extract_tx_hash, resolve_wallet, wallet_contract_call};
use crate::rpc::{
    erc20_decimals, fmt_token_amount, get_account_wei, human_to_atomic,
    native_balance, pad_u256, selectors, wait_for_tx,
};

/// Open an isolated borrow position and borrow against it.
///
/// Two on-chain transactions, both via BorrowPositionProxyV2 (the only proxy that
/// permits non-zero accounts to hold negative balances):
///
///   1. openBorrowPosition(0, N, collateralMarketId, collateralAmount, BalanceCheckFlag.Both=3)
///      — moves collateral from main account (0) to a new isolated position account (N).
///      Skipped if `--collateral-amount 0` is passed (re-borrowing against existing collateral).
///
///   2. transferBetweenAccounts(N, 0, borrowMarketId, borrowAmount, BalanceCheckFlag.To=2)
///      — pulls the borrow asset from account N (creating debt: account N's borrowMarket
///      balance becomes negative) and credits it as supply on account 0 (the main account).
///      To get the borrowed token into your wallet afterwards, run
///      `dolomite-plugin withdraw --token <borrow_token> --amount <X> --confirm`.
///
/// Why not use DepositWithdrawalProxy.withdrawWei: that proxy enforces non-negative
/// balances on all accounts regardless of BalanceCheckFlag (verified empirically —
/// even position accounts with valid collateralization revert with "account cannot go
/// negative"). Only BorrowPositionProxyV2 honors the flag for non-zero accounts.
///
/// Each isolated position can hold up to 32 collaterals.
#[derive(Args)]
pub struct BorrowArgs {
    /// Token to borrow (USDC / WETH / etc.) or 0x address
    #[arg(long)]
    pub token: String,
    /// Human-readable amount to borrow
    #[arg(long, allow_hyphen_values = true)]
    pub amount: String,
    /// Token to use as collateral. Must already be supplied to your main account.
    /// Optional when re-borrowing against an existing position (pair with `--collateral-amount 0`).
    #[arg(long)]
    pub collateral_token: Option<String>,
    /// Amount of collateral to move from main account into the position. Pass `0` to
    /// skip step 1 entirely and borrow against existing collateral in --position-account-number.
    #[arg(long, allow_hyphen_values = true, default_value = "0")]
    pub collateral_amount: String,
    /// Account number for the isolated position (1..=u128::MAX).
    /// Re-using an existing position account just adds to it.
    /// Default 100 — pick a fresh unused number for a brand new position.
    #[arg(long, default_value = "100")]
    pub position_account_number: u128,
    /// Dry run
    #[arg(long)]
    pub dry_run: bool,
    /// Required to submit
    #[arg(long)]
    pub confirm: bool,
    /// Tx confirmation timeout per step (default 180s)
    #[arg(long, default_value = "180")]
    pub timeout_secs: u64,
}

pub async fn run(args: BorrowArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0];

    if args.position_account_number == 0 {
        return print_err(
            "--position-account-number 0 is reserved for the main account; pick a number ≥ 1.",
            "INVALID_ARGUMENT",
            "Use --position-account-number 100 for your first borrow position.",
        );
    }

    // Resolve borrow token
    let (borrow_market, borrow_sym, borrow_addr) = match resolve_market_id(&args.token) {
        Some(t) => t,
        None => return print_err(
            &format!("Unknown borrow token '{}'", args.token),
            "TOKEN_NOT_FOUND",
            "Use one of USDC / USDT / WETH / DAI / WBTC / ARB / USDC.e / LINK, or 0x address.",
        ),
    };
    let borrow_decimals = token_decimals(borrow_sym)
        .or(erc20_decimals(borrow_addr, chain.rpc).await.ok())
        .unwrap_or(18);
    let borrow_amount_raw = match human_to_atomic(&args.amount, borrow_decimals) {
        Ok(v) => v,
        Err(e) => return print_err(&format!("Invalid --amount: {}", e), "INVALID_ARGUMENT",
            "Pass a positive number, e.g. --amount 1.0"),
    };

    // Parse --collateral-amount first; treat "0" / "0.0" as "skip step 1".
    let coll_amount_input = args.collateral_amount.trim();
    let skip_open = coll_amount_input == "0" || coll_amount_input == "0.0"
        || coll_amount_input.parse::<f64>().map(|v| v == 0.0).unwrap_or(false);

    // Collateral token only required when actually moving collateral.
    let (coll_market, coll_sym, coll_decimals, coll_amount_raw) = if skip_open {
        // Borrowing against existing collateral — collateral metadata is informational only.
        match args.collateral_token.as_deref() {
            Some(s) => match resolve_market_id(s) {
                Some((m, sym, addr)) => {
                    let dec = token_decimals(sym).or(erc20_decimals(addr, chain.rpc).await.ok()).unwrap_or(18);
                    (m, sym, dec, 0u128)
                }
                None => (0, "(none)", 18, 0u128),
            },
            None => (0, "(none)", 18, 0u128),
        }
    } else {
        let coll_token = match args.collateral_token.as_deref() {
            Some(s) => s,
            None => return print_err(
                "--collateral-token required when --collateral-amount > 0",
                "INVALID_ARGUMENT",
                "Pass --collateral-token USDC (or pass --collateral-amount 0 to borrow against existing position collateral).",
            ),
        };
        let (m, sym, addr) = match resolve_market_id(coll_token) {
            Some(t) => t,
            None => return print_err(
                &format!("Unknown collateral token '{}'", coll_token),
                "TOKEN_NOT_FOUND",
                "Use a supplied token, e.g. --collateral-token USDC.",
            ),
        };
        let dec = token_decimals(sym).or(erc20_decimals(addr, chain.rpc).await.ok()).unwrap_or(18);
        let raw = match human_to_atomic(coll_amount_input, dec) {
            Ok(v) => v,
            Err(e) => return print_err(&format!("Invalid --collateral-amount: {}", e), "INVALID_ARGUMENT",
                "Pass a positive number, e.g. --collateral-amount 0.5, or 0 to skip the open step."),
        };
        (m, sym, dec, raw)
    };

    let from_addr = match resolve_wallet(chain.id) {
        Ok(a) => a,
        Err(e) => return print_err(&format!("{:#}", e), "WALLET_NOT_FOUND",
            "Run `onchainos wallet addresses`."),
    };

    // Pre-flight: native gas (two write txs needed)
    let native = native_balance(&from_addr, chain.rpc).await
        .map_err(|e| anyhow::anyhow!("RPC: {}", e))?;
    // 0.0005 ETH floor — matches supply/withdraw/repay; Arbitrum L2 txs are
    // ~0.000005 ETH each so two-tx flow comfortably fits within this floor.
    if native < 500_000_000_000_000 {
        return print_err(
            &format!("Native ETH on Arbitrum is {} — borrow needs ≥0.0005 ETH for two txs.", fmt_token_amount(native, 18)),
            "INSUFFICIENT_GAS",
            "Top up at least 0.0005 ETH on Arbitrum.",
        );
    }

    // Pre-flight: main account must have ≥ collateral_amount of collateral_token (only if opening).
    // EVM-012: RPC failure must not be silently rendered as "supply=0" — that
    // would block a legitimate borrow when the user actually has the collateral.
    if !skip_open {
        let (main_sign, main_supply) = match get_account_wei(
            chain.dolomite_margin, &from_addr, 0, coll_market as u128, chain.rpc,
        ).await {
            Ok(t) => t,
            Err(e) => return print_err(
                &format!("Failed to read main-account collateral supply from DolomiteMargin on {}: {:#}", chain.key, e),
                "RPC_ERROR",
                "Public RPC may be limited; retry shortly.",
            ),
        };
        if !main_sign || main_supply < coll_amount_raw {
            return print_err(
                &format!(
                    "Main account has only {} {} supplied (raw {}); cannot move {} (raw {}) as collateral.",
                    fmt_token_amount(main_supply, coll_decimals), coll_sym, main_supply,
                    fmt_token_amount(coll_amount_raw, coll_decimals), coll_amount_raw,
                ),
                "INSUFFICIENT_COLLATERAL",
                "Reduce --collateral-amount or first `dolomite-plugin supply --token <X> --amount <Y> --confirm`.",
            );
        }
    }

    // Build calldata for step 1: openBorrowPosition(0, N, coll_market, coll_amount, BalanceCheckFlag.Both=3)
    let open_calldata = format!(
        "{}{}{}{}{}{}",
        selectors::OPEN_BORROW_POSITION,
        pad_u256(0),                                  // fromAccountNumber (main)
        pad_u256(args.position_account_number),       // toAccountNumber (isolated)
        pad_u256(coll_market as u128),
        pad_u256(coll_amount_raw),
        pad_u256(3),                                  // BalanceCheckFlag.Both — main can't go neg, position must accept inflow
    );

    // Build calldata for step 2: transferBetweenAccounts(N, 0, borrow_market, borrow_amount, BalanceCheckFlag.To=2)
    // — moves the borrow asset from position N (creating debt) to main account 0 (as supply).
    let transfer_calldata = format!(
        "{}{}{}{}{}{}",
        selectors::TRANSFER_BETWEEN_ACCTS,
        pad_u256(args.position_account_number),       // from (position; can go negative)
        pad_u256(0),                                  // to   (main; gains supply)
        pad_u256(borrow_market as u128),
        pad_u256(borrow_amount_raw),
        pad_u256(2),                                  // BalanceCheckFlag.To — only main must remain non-negative
    );

    let stage = if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" };
    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "stage": stage,
        "submitted": false,
        "preview": {
            "action": "borrow",
            "chain": chain.key,
            "from": from_addr,
            "borrow_token": borrow_sym,
            "borrow_market_id": borrow_market,
            "borrow_amount":     fmt_token_amount(borrow_amount_raw, borrow_decimals),
            "borrow_amount_raw": borrow_amount_raw.to_string(),
            "collateral_token": coll_sym,
            "collateral_market_id": coll_market,
            "collateral_amount":     fmt_token_amount(coll_amount_raw, coll_decimals),
            "collateral_amount_raw": coll_amount_raw.to_string(),
            "position_account_number": args.position_account_number,
            "step1_target": chain.borrow_position_proxy,
            "step2_target": chain.borrow_position_proxy,
            "warning": "Two-tx flow: (1) move collateral to position N, (2) transferBetweenAccounts(N→0) creates debt on N + supply on 0. Borrowed token lands as supply on main; run `withdraw` after to send to wallet.",
        }
    }))?);

    if args.dry_run { eprintln!("[DRY RUN] Calldata built; not signing."); return Ok(()); }
    if !args.confirm { eprintln!("[PREVIEW] Add --confirm to submit."); return Ok(()); }

    // ---- Step 1: openBorrowPosition (skipped if --collateral-amount 0) ----
    let open_hash: Option<String> = if skip_open {
        eprintln!("[borrow] Skipping step 1 (using existing collateral on account {}).", args.position_account_number);
        None
    } else {
        eprintln!("[borrow] Step 1: openBorrowPosition (move {} {} → account {})…",
            fmt_token_amount(coll_amount_raw, coll_decimals), coll_sym, args.position_account_number);
        let open_result = match wallet_contract_call(chain.id, chain.borrow_position_proxy, &open_calldata, None, Some(450_000), false) {
            Ok(r) => r,
            Err(e) => return print_err(
                &format!("openBorrowPosition failed: {:#}", e),
                "OPEN_POSITION_FAILED",
                "Common: insufficient supply on main account, gas, RPC.",
            ),
        };
        let h = match extract_tx_hash(&open_result) {
            Some(h) => h,
            None => return print_err("openBorrowPosition broadcast but no tx hash",
                "TX_HASH_MISSING", "Check `onchainos wallet history`."),
        };
        eprintln!("[borrow] openBorrowPosition tx: {} — waiting…", h);
        if let Err(e) = wait_for_tx(&h, chain.rpc, args.timeout_secs).await {
            return print_err(&format!("openBorrowPosition tx {} reverted: {:#}", h, e),
                "OPEN_POSITION_REVERTED",
                "Step 1 reverted — borrow not attempted. Inspect on Arbiscan.");
        }
        eprintln!("[borrow] Step 1 confirmed.");
        Some(h)
    };

    // ---- Step 2: transferBetweenAccounts (the actual borrow — creates debt on N, supply on 0) ----
    eprintln!("[borrow] Step 2: transferBetweenAccounts({} → 0, {} {}) — creates debt on position…",
        args.position_account_number, fmt_token_amount(borrow_amount_raw, borrow_decimals), borrow_sym);
    let borrow_result = match wallet_contract_call(chain.id, chain.borrow_position_proxy, &transfer_calldata, None, Some(450_000), false) {
        Ok(r) => r,
        Err(e) => return print_err(
            &format!("withdrawWei (borrow) failed: {:#}", e),
            "BORROW_SUBMIT_FAILED",
            "Common: undercollateralized — collateral too small, or LTV cap. Reduce --amount or increase --collateral-amount. Position has been opened (collateral moved); use `repay` if you want to clear and `withdraw --from-account-number N` to recover collateral, or retry borrow.",
        ),
    };
    let borrow_hash = match extract_tx_hash(&borrow_result) {
        Some(h) => h,
        None => return print_err("Borrow broadcast but no tx hash",
            "TX_HASH_MISSING", "Check `onchainos wallet history`."),
    };
    eprintln!("[borrow] borrow tx: {} — waiting…", borrow_hash);
    if let Err(e) = wait_for_tx(&borrow_hash, chain.rpc, args.timeout_secs).await {
        return print_err(&format!("Borrow tx {} reverted: {:#}", borrow_hash, e),
            "TX_REVERTED",
            "Most common: undercollateralization. Position retains collateral; consider closing it.");
    }
    eprintln!("[borrow] Step 2 confirmed (status 0x1).");

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "borrow",
        "chain": chain.key,
        "borrow_token": borrow_sym,
        "borrow_amount":     fmt_token_amount(borrow_amount_raw, borrow_decimals),
        "borrow_amount_raw": borrow_amount_raw.to_string(),
        "collateral_token": coll_sym,
        "collateral_amount":     fmt_token_amount(coll_amount_raw, coll_decimals),
        "collateral_amount_raw": coll_amount_raw.to_string(),
        "position_account_number": args.position_account_number,
        "open_position_tx": open_hash,
        "open_position_skipped": open_hash.is_none(),
        "borrow_tx": borrow_hash,
        "on_chain_status": "0x1",
        "tip": format!(
            "Borrowed {} {} now sits as supply on main account (0). Run `dolomite-plugin withdraw --token {} --amount {} --confirm` to move it to your wallet. Position {} now has {} {} debt — to close: `dolomite-plugin repay --token {} --all --position-account-number {} --confirm`.",
            fmt_token_amount(borrow_amount_raw, borrow_decimals), borrow_sym,
            borrow_sym, fmt_token_amount(borrow_amount_raw, borrow_decimals),
            args.position_account_number,
            fmt_token_amount(borrow_amount_raw, borrow_decimals), borrow_sym,
            borrow_sym, args.position_account_number,
        ),
    }))?);
    Ok(())
}

fn print_err(msg: &str, code: &str, suggestion: &str) -> anyhow::Result<()> {
    println!("{}", super::error_response(msg, code, suggestion));
    Ok(())
}
