use clap::Args;

use crate::api;
use crate::config::KVAULT_PROGRAM_ID;
use crate::onchainos;

#[derive(Args, Debug)]
pub struct WithdrawArgs {
    /// Chain ID (must be 501 for Solana)
    #[arg(long, default_value = "501")]
    pub chain: u64,

    /// KVault address (base58) to withdraw from
    #[arg(long)]
    pub vault: String,

    /// Amount of shares to redeem (UI units, e.g. "1" = 1 share)
    #[arg(long)]
    pub amount: String,

    /// Wallet address (base58). If omitted, resolved from onchainos.
    #[arg(long)]
    pub wallet: Option<String>,

    /// Dry run — simulate without broadcasting
    #[arg(long)]
    pub dry_run: bool,
    /// Confirm and broadcast the transaction (without this flag, prints a preview only)
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: WithdrawArgs) -> anyhow::Result<()> {
    if args.chain != 501 {
        anyhow::bail!("kamino-liquidity only supports Solana (chain 501)");
    }

    // Dry-run early return — before wallet resolution
    if args.dry_run {
        let dummy_wallet = "DTEqFXyFM9aMSGu9sw3PpRsZce6xqqmaUbGkFjmeieGE";
        let wallet = args.wallet.as_deref().unwrap_or(dummy_wallet);
        let tx_b64 = api::build_withdraw_tx(&args.vault, wallet, &args.amount).await?;
        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "vault": args.vault,
            "amount": args.amount,
            "serialized_tx": tx_b64,
            "data": { "txHash": "" }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Resolve wallet (after dry-run guard)
    let wallet = match args.wallet {
        Some(w) => w,
        None => onchainos::resolve_wallet_solana()?,
    };

    if wallet.is_empty() {
        anyhow::bail!("Could not resolve wallet address. Pass --wallet <address> or ensure onchainos is logged in.");
    }

    // Build withdraw transaction from Kamino API
    // NOTE: amount = shares in UI units (not token amount)
    let tx_b64 = api::build_withdraw_tx(&args.vault, &wallet, &args.amount).await?;

    // Submit via onchainos (base64→base58 conversion done internally)
    // Solana blockhash expires ~60s — must submit immediately
    // ── Preview mode: show TX details without broadcasting ──────────────────
    if !args.confirm && !args.dry_run {
        println!("=== Transaction Preview (NOT broadcast) ===");
        println!("Add --confirm to execute this transaction.");
        return Ok(());
    }
    let result = onchainos::wallet_contract_call_solana(KVAULT_PROGRAM_ID, &tx_b64, false).await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let output = serde_json::json!({
        "ok": true,
        "vault": args.vault,
        "wallet": wallet,
        "shares_redeemed": args.amount,
        "data": {
            "txHash": tx_hash
        },
        "explorer": format!("https://solscan.io/tx/{}", tx_hash)
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
