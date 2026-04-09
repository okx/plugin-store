use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{DEFAULT_SLIPPAGE_BPS, DEFAULT_TX_VERSION, TX_API_BASE};

#[derive(Args, Debug)]
pub struct GetSwapQuoteArgs {
    /// Input token mint address
    #[arg(long)]
    pub input_mint: String,

    /// Output token mint address
    #[arg(long)]
    pub output_mint: String,

    /// Input amount in base units (with decimals)
    #[arg(long)]
    pub amount: u64,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u32,

    /// Transaction version: V0 or LEGACY (default: V0)
    #[arg(long, default_value = DEFAULT_TX_VERSION)]
    pub tx_version: String,
}

pub async fn execute(args: &GetSwapQuoteArgs) -> Result<()> {
    crate::config::validate_solana_address(&args.input_mint)?;
    crate::config::validate_solana_address(&args.output_mint)?;

    let client = reqwest::Client::new();
    let url = format!("{}/compute/swap-base-in", TX_API_BASE);
    let resp: Value = client
        .get(&url)
        .query(&[
            ("inputMint", args.input_mint.as_str()),
            ("outputMint", args.output_mint.as_str()),
            ("amount", &args.amount.to_string()),
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
