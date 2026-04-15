use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::config::DEFAULT_SLIPPAGE_BPS;
use crate::onchainos::{self, SOL_MINT};

#[derive(Args, Debug)]
pub struct BuyArgs {
    /// Token mint address (base58)
    #[arg(long)]
    pub mint: String,

    /// SOL amount to spend, in readable units (e.g. "0.01" = 0.01 SOL)
    #[arg(long)]
    pub sol_amount: String,

    /// Slippage tolerance in basis points (default: 100 = 1%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u64,

    /// Confirm execution — required to execute on-chain. Without this flag, shows a preview.
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Serialize, Debug)]
struct BuyOutput {
    ok: bool,
    mint: String,
    sol_amount: String,
    slippage_bps: u64,
    tx_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

pub async fn execute(args: &BuyArgs, dry_run: bool) -> Result<()> {
    if dry_run || !args.confirm {
        let (is_dry_run, is_preview, note) = if dry_run {
            (Some(true), None, "dry_run=true — no transaction submitted. Pass --confirm to execute.".to_string())
        } else {
            (None, Some(true), "Preview: re-run with --confirm to execute on-chain.".to_string())
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&BuyOutput {
                ok: true,
                mint: args.mint.clone(),
                sol_amount: args.sol_amount.clone(),
                slippage_bps: args.slippage_bps,
                tx_hash: String::new(),
                dry_run: is_dry_run,
                preview: is_preview,
                note: Some(note),
            })?
        );
        return Ok(());
    }

    let result =
        onchainos::swap_execute_solana(SOL_MINT, &args.mint, &args.sol_amount, args.slippage_bps)
            .await?;

    let tx_hash = onchainos::extract_tx_hash(&result)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&BuyOutput {
            ok: true,
            mint: args.mint.clone(),
            sol_amount: args.sol_amount.clone(),
            slippage_bps: args.slippage_bps,
            tx_hash,
            dry_run: None,
            preview: None,
            note: None,
        })?
    );
    Ok(())
}
