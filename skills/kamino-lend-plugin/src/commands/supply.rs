use clap::Args;

use crate::{api, config, onchainos};

#[derive(Args)]
pub struct SupplyArgs {
    /// Token symbol (e.g., USDC, SOL) or reserve address
    #[arg(long)]
    pub token: String,

    /// Amount to supply in UI units (e.g., 0.01 for 0.01 USDC)
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

pub async fn run(args: SupplyArgs) -> anyhow::Result<()> {
    // Resolve reserve early — validates token even in dry-run
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
                    "action": "supply"
                }
            }))?
        );
        return Ok(());
    }

    // Resolve wallet (must be done AFTER dry-run guard)
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

    // SOL/wSOL deposit requires an existing obligation account.
    // The Kamino API cannot create the obligation and wrap SOL in the same transaction.
    // Check upfront and give a clear error rather than a cryptic on-chain simulation failure.
    if is_sol_token(&args.token) {
        let obligations = api::get_obligations(&market, &wallet).await.unwrap_or_default();
        if obligations.as_array().map(|a| a.is_empty()).unwrap_or(true) {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": "SOL deposit requires an existing Kamino obligation account.",
                    "error_code": "NO_OBLIGATION",
                    "suggestion": "Supply USDC first to initialize your account, then SOL deposits will work: kamino-lend supply --token USDC --amount <amount> --confirm"
                }))?
            );
            return Ok(());
        }
    }

    // Build transaction via Kamino API — returns base64 serialized tx
    let tx_b64 = match api::build_deposit_tx(&wallet, &market, &reserve, &args.amount).await {
        Ok(tx) => tx,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.token)));
            return Ok(());
        }
    };

    // Submit via onchainos (converts base64 → base58 internally)
    // ── Preview mode: show TX details without broadcasting ──────────────────
    if !args.confirm && !args.dry_run {
        println!("=== Transaction Preview (NOT broadcast) ===");
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
                "action": "supply",
                "explorer": format!("https://solscan.io/tx/{}", tx_hash)
            }
        }))?
    );

    Ok(())
}

/// Returns true for SOL and wSOL (both map to the SOL reserve, both require
/// an existing obligation account before the Kamino API can build the deposit tx).
fn is_sol_token(token: &str) -> bool {
    matches!(token.to_uppercase().as_str(), "SOL" | "WSOL")
}

fn resolve_reserve(token_or_address: &str) -> anyhow::Result<String> {
    // If it looks like a base58 address (32+ chars), use directly
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
