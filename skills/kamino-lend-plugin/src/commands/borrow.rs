use clap::Args;

use crate::{api, config, onchainos};

#[derive(Args)]
pub struct BorrowArgs {
    /// Token symbol (e.g., USDC, SOL) or reserve address
    #[arg(long)]
    pub token: String,

    /// Amount to borrow in UI units (e.g., 0.001 for 0.001 SOL)
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

pub async fn run(args: BorrowArgs) -> anyhow::Result<()> {
    let reserve = resolve_reserve(&args.token)?;

    // Borrow is dry-run only per GUARDRAILS (liquidation risk with limited funds)
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
                    "action": "borrow"
                },
                "note": "Borrow requires prior supply as collateral. Use --dry-run to preview."
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

    // Build transaction via Kamino API
    let tx_b64 = match api::build_borrow_tx(&wallet, &market, &reserve, &args.amount).await {
        Ok(tx) => tx,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.token)));
            return Ok(());
        }
    };

    // Submit via onchainos
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
                "action": "borrow",
                "explorer": format!("https://solscan.io/tx/{}", tx_hash)
            }
        }))?
    );

    Ok(())
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
