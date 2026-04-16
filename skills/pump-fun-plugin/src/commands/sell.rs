use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::config::DEFAULT_SLIPPAGE_BPS;
use crate::onchainos::{self, SOL_MINT};
use crate::onchainos::resolve_wallet_solana;

#[derive(Args, Debug)]
pub struct SellArgs {
    /// Token mint address (base58)
    #[arg(long)]
    pub mint: String,

    /// Readable token amount to sell (e.g. "1000000"). Omit to sell entire balance.
    #[arg(long)]
    pub token_amount: Option<String>,

    /// Slippage tolerance in basis points (default: 100 = 1%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u64,

    /// Confirm execution — required to execute on-chain. Without this flag, shows a preview.
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Serialize, Debug)]
struct SellOutput {
    ok: bool,
    mint: String,
    token_amount: String,
    slippage_bps: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    wallet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    explorer_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

pub async fn execute(args: &SellArgs, dry_run: bool) -> Result<()> {
    // Resolve amount: explicit or placeholder/balance
    let amount = match &args.token_amount {
        Some(a) => a.clone(),
        None => {
            if dry_run || !args.confirm {
                "<full balance>".to_string()
            } else {
                onchainos::get_token_balance(&args.mint)?
                    .ok_or_else(|| anyhow::anyhow!(
                        "No balance found for mint {} in your Solana wallet. \
                         Ensure the token is held in the active wallet, or specify \
                         the amount explicitly with --token-amount <amount>.",
                        args.mint
                    ))?
            }
        }
    };

    if dry_run || !args.confirm {
        let wallet = resolve_wallet_solana().ok();
        let (is_dry_run, is_preview, note) = if dry_run {
            (Some(true), None, "dry_run=true — no transaction submitted. Pass --confirm to execute.".to_string())
        } else {
            (None, Some(true), "Preview: re-run with --confirm to execute on-chain.".to_string())
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&SellOutput {
                ok: true,
                mint: args.mint.clone(),
                token_amount: amount,
                slippage_bps: args.slippage_bps,
                wallet,
                tx_hash: None,
                explorer_url: None,
                dry_run: is_dry_run,
                preview: is_preview,
                note: Some(note),
            })?
        );
        return Ok(());
    }

    let result =
        onchainos::swap_execute_solana(&args.mint, SOL_MINT, &amount, args.slippage_bps).await?;

    let tx_hash = onchainos::extract_tx_hash(&result)?;
    let wallet = resolve_wallet_solana().ok();
    let explorer_url = Some(format!("https://solscan.io/tx/{}", tx_hash));

    println!(
        "{}",
        serde_json::to_string_pretty(&SellOutput {
            ok: true,
            mint: args.mint.clone(),
            token_amount: amount,
            slippage_bps: args.slippage_bps,
            wallet,
            tx_hash: Some(tx_hash),
            explorer_url,
            dry_run: None,
            preview: None,
            note: None,
        })?
    );
    Ok(())
}
