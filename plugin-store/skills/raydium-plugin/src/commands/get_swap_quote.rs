use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{
    parse_human_amount, DEFAULT_SLIPPAGE_BPS, DEFAULT_TX_VERSION, SOL_NATIVE_MINT,
    SOL_SYSTEM_PROGRAM, USDC_SOLANA, TX_API_BASE,
};

#[derive(Args, Debug)]
pub struct GetSwapQuoteArgs {
    /// Input token mint address
    #[arg(long)]
    pub input_mint: String,

    /// Output token mint address
    #[arg(long)]
    pub output_mint: String,

    /// Input amount in human-readable units (e.g. "0.1" for 0.1 SOL, "1.5" for 1.5 USDC)
    #[arg(long)]
    pub amount: String,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u32,

    /// Transaction version: V0 or LEGACY (default: V0)
    #[arg(long, default_value = DEFAULT_TX_VERSION)]
    pub tx_version: String,
}

/// Resolve decimals for well-known Solana mints, falling back to Raydium mint API.
async fn resolve_decimals(mint: &str, client: &reqwest::Client) -> anyhow::Result<u8> {
    if mint == SOL_NATIVE_MINT || mint == SOL_SYSTEM_PROGRAM {
        return Ok(9);
    }
    if mint == USDC_SOLANA {
        return Ok(6);
    }
    let url = format!("{}/mint/ids", crate::config::DATA_API_BASE);
    let resp: Value = client
        .get(&url)
        .query(&[("mints", mint)])
        .send()
        .await?
        .json()
        .await?;
    if let Some(decimals) = resp["data"][0]["decimals"].as_u64() {
        return Ok(decimals as u8);
    }
    anyhow::bail!("Could not resolve decimals for mint '{}'", mint)
}

pub async fn execute(args: &GetSwapQuoteArgs) -> Result<()> {
    // Rewrite native SOL system program address to WSOL — Raydium routes use WSOL
    let input_mint = if args.input_mint == SOL_SYSTEM_PROGRAM {
        SOL_NATIVE_MINT.to_string()
    } else {
        args.input_mint.clone()
    };
    let output_mint = if args.output_mint == SOL_SYSTEM_PROGRAM {
        SOL_NATIVE_MINT.to_string()
    } else {
        args.output_mint.clone()
    };

    crate::config::validate_solana_address(&input_mint)?;
    crate::config::validate_solana_address(&output_mint)?;

    let client = reqwest::Client::new();

    let input_decimals = resolve_decimals(&input_mint, &client).await?;
    let raw_amount = parse_human_amount(&args.amount, input_decimals)?;

    let url = format!("{}/compute/swap-base-in", TX_API_BASE);
    let resp: Value = client
        .get(&url)
        .query(&[
            ("inputMint", input_mint.as_str()),
            ("outputMint", output_mint.as_str()),
            ("amount", &raw_amount.to_string()),
            ("slippageBps", &args.slippage_bps.to_string()),
            ("txVersion", args.tx_version.as_str()),
        ])
        .send()
        .await?
        .json()
        .await?;

    // Surface API errors as structured JSON with exit 1
    if resp.get("success").and_then(|v| v.as_bool()) == Some(false) {
        let msg = resp["msg"].as_str().unwrap_or("Raydium API error");
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": msg,
                "raw": resp
            }))?
        );
        std::process::exit(1);
    }

    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
