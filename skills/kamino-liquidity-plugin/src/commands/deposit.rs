use clap::Args;

use crate::api;
use crate::config::KVAULT_PROGRAM_ID;
use crate::onchainos;

#[derive(Args, Debug)]
pub struct DepositArgs {
    /// Chain ID (must be 501 for Solana)
    #[arg(long, default_value = "501")]
    pub chain: u64,

    /// KVault address (base58) to deposit into
    #[arg(long)]
    pub vault: String,

    /// Amount to deposit in UI units (e.g. 0.001 for 0.001 SOL)
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

pub async fn run(args: DepositArgs) -> anyhow::Result<()> {
    if args.chain != 501 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "kamino-liquidity only supports Solana (chain 501)",
                "error_code": "UNSUPPORTED_CHAIN",
                "suggestion": "Use --chain 501 or omit --chain (defaults to 501)."
            }))?
        );
        return Ok(());
    }

    // Dry-run early return — before wallet resolution
    if args.dry_run {
        let dummy_wallet = "DTEqFXyFM9aMSGu9sw3PpRsZce6xqqmaUbGkFjmeieGE";
        let wallet = args.wallet.as_deref().unwrap_or(dummy_wallet);
        let tx_b64 = match api::build_deposit_tx(&args.vault, wallet, &args.amount).await {
            Ok(tx) => tx,
            Err(e) => {
                println!("{}", super::error_response(&e, Some(&args.vault)));
                return Ok(());
            }
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "vault": args.vault,
                "amount": args.amount,
                "serialized_tx": tx_b64,
                "data": { "txHash": "" }
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
                println!("{}", super::error_response(&e, Some(&args.vault)));
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

    // Build deposit transaction from Kamino API
    let tx_b64 = match api::build_deposit_tx(&args.vault, &wallet, &args.amount).await {
        Ok(tx) => tx,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.vault)));
            return Ok(());
        }
    };

    // Preview mode
    if !args.confirm && !args.dry_run {
        println!("=== Transaction Preview (NOT broadcast) ===");
        println!("Add --confirm to execute this transaction.");
        return Ok(());
    }

    // Submit via onchainos (base64→base58 conversion done internally)
    // Solana blockhash expires ~60s — must submit immediately
    let result = match onchainos::wallet_contract_call_solana(KVAULT_PROGRAM_ID, &tx_b64, false).await {
        Ok(r) => r,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.vault)));
            return Ok(());
        }
    };

    let tx_hash = match onchainos::extract_tx_hash(&result) {
        Ok(h) => h,
        Err(e) => {
            println!("{}", super::error_response(&e, Some(&args.vault)));
            return Ok(());
        }
    };

    if let Err(e) = onchainos::wait_for_tx_solana(&tx_hash, &wallet).await {
        println!("{}", super::error_response(&e, Some(&args.vault)));
        return Ok(());
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "vault": args.vault,
            "wallet": wallet,
            "amount": args.amount,
            "action": "deposit",
            "data": {
                "txHash": tx_hash
            },
            "explorer": format!("https://solscan.io/tx/{}", tx_hash)
        }))?
    );
    Ok(())
}
