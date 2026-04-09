/// get-price: Compute the price ratio between two tokens using the swap quote endpoint.
/// Uses amount=1_000_000 (suitable for 6-decimal tokens) and divides outputAmount/inputAmount.
use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{DEFAULT_SLIPPAGE_BPS, DEFAULT_TX_VERSION, TX_API_BASE};

#[derive(Args, Debug)]
pub struct GetPriceArgs {
    /// Input token mint address (token you're selling)
    #[arg(long)]
    pub input_mint: String,

    /// Output token mint address (token you're buying)
    #[arg(long)]
    pub output_mint: String,

    /// Input amount in base units for price calculation (default: 1000000 = 1 unit for 6-decimal tokens)
    #[arg(long, default_value_t = 1_000_000)]
    pub amount: u64,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u32,

    /// Transaction version: V0 or LEGACY (default: V0)
    #[arg(long, default_value = DEFAULT_TX_VERSION)]
    pub tx_version: String,
}

pub async fn execute(args: &GetPriceArgs) -> Result<()> {
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

    // Compute price ratio from quote data
    let price_info = if let Some(data) = resp.get("data") {
        let input_amount: f64 = data["inputAmount"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(args.amount as f64);
        let output_amount: f64 = data["outputAmount"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let price_impact_pct = data["priceImpactPct"].as_f64().unwrap_or(0.0);
        let price = if input_amount > 0.0 {
            output_amount / input_amount
        } else {
            0.0
        };
        serde_json::json!({
            "inputMint": args.input_mint,
            "outputMint": args.output_mint,
            "price": price,
            "priceImpactPct": price_impact_pct,
            "inputAmount": input_amount,
            "outputAmount": output_amount,
            "quote": data,
        })
    } else {
        resp.clone()
    };

    println!("{}", serde_json::to_string_pretty(&price_info)?);
    Ok(())
}
