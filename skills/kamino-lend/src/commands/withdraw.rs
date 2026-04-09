use clap::Args;

use crate::{api, config, onchainos};

#[derive(Args)]
pub struct WithdrawArgs {
    /// Token symbol (e.g., USDC, SOL) or reserve address
    #[arg(long)]
    pub token: String,

    /// Amount to withdraw in UI units (e.g., 0.01 for 0.01 USDC)
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

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
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
                    "action": "withdraw"
                }
            }))?
        );
        return Ok(());
    }

    // Resolve wallet (after dry-run guard)
    let wallet = match args.wallet {
        Some(w) => w,
        None => onchainos::resolve_wallet_solana()?,
    };
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --wallet or ensure onchainos is logged in.");
    }

    let market = args.market.as_deref().unwrap_or(config::MAIN_MARKET).to_string();

    let reserve = resolve_reserve(&args.token)?;

    // Build transaction via Kamino API
    let tx_b64 = api::build_withdraw_tx(&wallet, &market, &reserve, &args.amount).await?;

    // Submit via onchainos
    // ── Preview mode: show TX details without broadcasting ──────────────────
    if !args.confirm && !args.dry_run {
        println!("=== Transaction Preview (NOT broadcast) ===");
        println!("Add --confirm to execute this transaction.");
        return Ok(());
    }
    let result = onchainos::wallet_contract_call_solana(
        config::KLEND_PROGRAM_ID,
        &tx_b64,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result)?;

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
                "action": "withdraw",
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
