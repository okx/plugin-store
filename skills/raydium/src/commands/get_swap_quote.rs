use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{
    parse_human_amount, DEFAULT_SLIPPAGE_BPS, DEFAULT_TX_VERSION, SOL_NATIVE_MINT, USDC_SOLANA,
    TX_API_BASE,
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
    if mint == SOL_NATIVE_MINT {
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
    crate::config::validate_solana_address(&args.input_mint)?;
    crate::config::validate_solana_address(&args.output_mint)?;

    let client = reqwest::Client::new();

    let input_decimals = resolve_decimals(&args.input_mint, &client).await?;
    let raw_amount = parse_human_amount(&args.amount, input_decimals)?;

    let url = format!("{}/compute/swap-base-in", TX_API_BASE);
    let resp: Value = client
        .get(&url)
        .query(&[
            ("inputMint", args.input_mint.as_str()),
            ("outputMint", args.output_mint.as_str()),
            ("amount", &raw_amount.to_string()),
            ("slippageBps", &args.slippage_bps.to_string()),
            ("txVersion", args.tx_version.as_str()),
        ])
        .send()
        .await?
        .json()
        .await?;

    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}
