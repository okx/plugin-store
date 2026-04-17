use clap::Args;

use crate::{api, config, onchainos};

#[derive(Args)]
pub struct RepayArgs {
    /// Token symbol (e.g., USDC, SOL) or reserve address
    #[arg(long)]
    pub token: String,

    /// Amount to repay in UI units, or "all"/"max" to repay full debt
    #[arg(long)]
    pub amount: String,

    /// Market address (optional; defaults to main market)
    #[arg(long)]
    pub market: Option<String>,

    /// Wallet address (optional; defaults to current onchainos Solana wallet)
    #[arg(long)]
    pub wallet: Option<String>,

    /// Dry-run mode: simulate without submitting transaction
    #[arg(long, default_value = "false")]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

/// Sentinel amount used when we want Kamino to repay the full outstanding debt.
/// Kamino's repay instruction uses min(amount_passed, current_debt) on-chain,
/// so any amount larger than the actual debt safely closes the full position.
const REPAY_ALL_SENTINEL: &str = "1000000000.0";

pub async fn run(args: RepayArgs) -> anyhow::Result<()> {
    let reserve = resolve_reserve(&args.token)?;

    if args.dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "txHash": "",
                    "token": args.token,
                    "amount": args.amount,
                    "reserve": reserve,
                    "action": "repay"
                }
            }))?
        );
        return Ok(());
    }

    // Resolve wallet (after dry-run guard)
    let wallet = match args.wallet {
        Some(w) => w,
        None => match onchainos::resolve_wallet_solana() {
            Ok(w) => w,
            Err(e) => {
                println!("{}", super::error_response(&e, Some(&args.token)));
                return Ok(());
            }
        },
    };
    if wallet.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "Cannot resolve wallet address.",
                "error_code": "WALLET_NOT_FOUND",
                "suggestion": "Pass --wallet <address> or run `onchainos wallet balance --chain 501` to verify login."
            }))?
        );
        return Ok(());
    }

    let market = args.market.as_deref().unwrap_or(config::MAIN_MARKET).to_string();

    // Determine effective repay amount, auto-upgrading to "repay all" when needed.
    let (effective_amount, repay_all, auto_swapped) = match resolve_effective_amount(
        &args.amount,
        &reserve,
        &market,
        &wallet,
    )
    .await
    {
        Ok(result) => result,
        Err(preflight_err) => {
            println!("{}", serde_json::to_string_pretty(&preflight_err)?);
            return Ok(());
        }
    };

    // Build transaction via Kamino API
    let tx_b64 = match api::build_repay_tx(&wallet, &market, &reserve, &effective_amount).await {
        Ok(tx) => tx,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.token)));
            return Ok(());
        }
    };

    // ── Preview mode: show TX details without broadcasting ──────────────────
    if !args.confirm && !args.dry_run {
        println!("=== Transaction Preview (NOT broadcast) ===");
        if repay_all {
            println!("Note: amount upgraded to 'repay all' — outstanding debt includes accrued interest.");
        }
        println!("Add --confirm to execute this transaction.");
        return Ok(());
    }

    let result = match onchainos::wallet_contract_call_solana(
        config::KLEND_PROGRAM_ID,
        &tx_b64,
        false,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.token)));
            return Ok(());
        }
    };

    let tx_hash = match onchainos::extract_tx_hash(&result) {
        Ok(h) => h,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.token)));
            return Ok(());
        }
    };

    if let Err(e) = onchainos::wait_for_tx_solana(&tx_hash, &wallet).await {
        println!("{}", super::error_response(&e, Some(&args.token)));
        return Ok(());
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "txHash": tx_hash,
                "token": args.token,
                "amount": args.amount,
                "market": market,
                "reserve": reserve,
                "action": "repay",
                "note": if auto_swapped {
                    "repaid full outstanding debt; auto-swapped 0.001 SOL via Jupiter to cover accrued interest shortfall"
                } else if repay_all {
                    "repaid full outstanding debt (auto-adjusted for accrued interest)"
                } else { "" },
                "auto_swap": auto_swapped,
                "explorer": format!("https://solscan.io/tx/{}", tx_hash)
            }
        }))?
    );

    Ok(())
}

/// Decide the actual amount string to send to the Kamino API.
///
/// Logic:
/// 1. Explicit "all"/"max" → resolve wallet-aware full-repay amount.
/// 2. Numeric amount → fetch current debt for this reserve.
///    If user amount >= 90% of current debt, treat as "repay all".
///    Otherwise use the exact user amount (partial repay).
///
/// Returns Err(json) for pre-flight failures (e.g. wallet short due to interest accrual).
/// Falls back to sentinel on API errors so we never silently block when debt can't be fetched.
async fn resolve_effective_amount(
    user_amount: &str,
    reserve: &str,
    market: &str,
    wallet: &str,
) -> Result<(String, bool, bool), serde_json::Value> {
    let is_repay_all = user_amount.eq_ignore_ascii_case("all")
        || user_amount.eq_ignore_ascii_case("max");

    let user_f: Option<f64> = if !is_repay_all {
        user_amount.parse::<f64>().ok()
    } else {
        None
    };

    // Fetch current debt
    let debt = fetch_debt_for_reserve(market, wallet, reserve).await;

    let should_repay_all = is_repay_all || {
        if let (Some(uf), Some((debt_f, _))) = (user_f, debt) {
            debt_f > 0.0 && uf >= debt_f * 0.9
        } else {
            false
        }
    };

    if !should_repay_all {
        return Ok((user_amount.to_string(), false, false));
    }

    // Full repay intent: check wallet has enough before proceeding.
    // Kamino requires repaying the EXACT full debt; partial repays leaving tiny dust
    // are rejected on-chain with "Net value remaining too small".
    if let Some((debt_f, debt_raw)) = debt {
        let decimals = config::reserve_decimals(reserve);
        let token_sym = config::reserve_symbol(reserve);

        if let Some(wallet_raw) = fetch_wallet_balance_raw(reserve, decimals) {
            if wallet_raw < debt_raw {
                // Wallet is short (accrued interest). Attempt auto-swap: SOL → token.
                let shortfall = debt_raw - wallet_raw;
                eprintln!(
                    "[kamino-lend] Wallet is {} atom(s) short of debt ({} {}). \
                     Auto-swapping 0.001 SOL → {} via Jupiter...",
                    shortfall, token_sym, token_sym, token_sym
                );

                let swapped = if let Some(mint) = config::reserve_mint(reserve) {
                    match api::jupiter_swap_sol_to_token(wallet, mint, 1_000_000).await {
                        Ok(swap_tx_b64) => {
                            match crate::onchainos::wallet_contract_call_solana(
                                api::JUPITER_PROGRAM_ID,
                                &swap_tx_b64,
                                false,
                            ).await {
                                Ok(_) => {
                                    eprintln!("[kamino-lend] Swap submitted. Waiting for confirmation...");
                                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                    true
                                }
                                Err(e) => {
                                    eprintln!("[kamino-lend] Swap tx failed: {}", e);
                                    false
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("[kamino-lend] Jupiter quote failed: {}", e);
                            false
                        }
                    }
                } else {
                    false
                };

                if !swapped {
                    // Auto-swap failed — fall through to structured error
                    let debt_ui_str = format!("{:.prec$}", debt_f, prec = (decimals as usize).min(9));
                    let wallet_ui_str = format!(
                        "{:.prec$}",
                        wallet_raw as f64 / 10f64.powi(decimals as i32),
                        prec = (decimals as usize).min(9)
                    );
                    return Err(serde_json::json!({
                        "ok": false,
                        "error": format!(
                            "Wallet is {} atom(s) short of outstanding {} debt due to accrued interest, \
                             and auto-swap failed.",
                            shortfall, token_sym
                        ),
                        "error_code": "INTEREST_SHORTFALL",
                        "data": {
                            "token": token_sym,
                            "debt": debt_ui_str,
                            "wallet_balance": wallet_ui_str,
                            "shortfall_atoms": shortfall,
                        },
                        "suggestion": format!(
                            "Need {} {} to repay but wallet only holds {} ({} atom(s) short). \
                             Please swap a small amount of SOL → {} first (e.g. 0.001 SOL), then repay again.",
                            debt_ui_str, token_sym, wallet_ui_str, shortfall, token_sym
                        )
                    }));
                }

                // Re-fetch balance after swap
                if let Some(new_wallet_raw) = fetch_wallet_balance_raw(reserve, decimals) {
                    let effective_raw = debt_raw.min(new_wallet_raw);
                    let effective_ui = format!(
                        "{:.prec$}",
                        effective_raw as f64 / 10f64.powi(decimals as i32),
                        prec = (decimals as usize).min(9)
                    );
                    eprintln!("[kamino-lend] Balance confirmed. Proceeding with repay.");
                    return Ok((effective_ui, true, true)); // auto_swapped = true
                }
            }
        }

        // Wallet has enough — repay exact debt amount (not sentinel) for precision
        eprintln!(
            "[kamino-lend] Note: outstanding debt is {:.8} (inc. accrued interest); \
             repaying full amount.",
            debt_f
        );
        let debt_ui_str = format!(
            "{:.prec$}", debt_f, prec = (decimals as usize).min(9)
        );
        return Ok((debt_ui_str, true, false));
    }

    // Could not fetch debt — fall back to sentinel (works if wallet has enough)
    Ok((REPAY_ALL_SENTINEL.to_string(), true, false))
}

/// Fetch the current outstanding debt for a specific reserve from obligations.
/// Returns (ui_amount, raw_atoms). None on any error (API down, no obligation, zero debt).
///
/// API path: obligations[].state.borrows[].borrowReserve / borrowedAmountOutsideElevationGroups
async fn fetch_debt_for_reserve(market: &str, wallet: &str, reserve: &str) -> Option<(f64, u64)> {
    let obligations = api::get_obligations(market, wallet).await.ok()?;
    let arr = obligations.as_array()?;
    let decimals = config::reserve_decimals(reserve);

    for obl in arr {
        let state = obl.get("state")?;
        let borrows = state["borrows"].as_array()?;
        for borrow in borrows {
            let r = borrow["borrowReserve"].as_str().unwrap_or("");
            if r != reserve {
                continue;
            }
            let raw = borrow["borrowedAmountOutsideElevationGroups"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            if raw == 0 {
                continue;
            }
            let ui = raw as f64 / 10f64.powi(decimals as i32);
            return Some((ui, raw));
        }
    }
    None
}

/// Get wallet token balance in raw atoms for the token associated with a reserve.
/// Calls `onchainos wallet balance` once and matches by symbol (case-insensitive).
/// Handles common wallet aliases: ETH↔WETH, SOL↔WSOL.
fn fetch_wallet_balance_raw(reserve: &str, decimals: u32) -> Option<u64> {
    let symbol = config::reserve_symbol(reserve);
    if symbol == "UNKNOWN" {
        return None;
    }
    let balances = crate::onchainos::get_all_token_balances();
    let balance_ui = balances
        .iter()
        .find(|(sym, _, _)| {
            sym.eq_ignore_ascii_case(symbol)
                // onchainos labels Wormhole ETH as "WETH"; config stores it as "ETH"
                || (symbol == "ETH" && sym.eq_ignore_ascii_case("WETH"))
                || (symbol == "SOL" && sym.eq_ignore_ascii_case("WSOL"))
        })
        .map(|(_, bal, _)| *bal)?;
    Some((balance_ui * 10f64.powi(decimals as i32)).round() as u64)
}

fn resolve_reserve(token_or_address: &str) -> anyhow::Result<String> {
    if token_or_address.len() > 30 {
        return Ok(token_or_address.to_string());
    }
    config::reserve_address(token_or_address)
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown token '{}'. Use a known symbol (USDC, SOL) or pass the reserve address directly.",
                token_or_address
            )
        })
}
