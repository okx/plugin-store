/// get-price: Compute the price ratio between two tokens using the swap quote endpoint.
/// Uses amount="1" (one full token unit) and divides outputAmount/inputAmount.
use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{
    parse_human_amount, DEFAULT_SLIPPAGE_BPS, DEFAULT_TX_VERSION, SOL_NATIVE_MINT,
    SOL_SYSTEM_PROGRAM, USDC_SOLANA, TX_API_BASE,
};

#[derive(Args, Debug)]
pub struct GetPriceArgs {
    /// Input token mint address (token you're selling)
    #[arg(long)]
    pub input_mint: String,

    /// Output token mint address (token you're buying)
    #[arg(long)]
    pub output_mint: String,

    /// Input amount in human-readable units for price calculation (default: "1" = 1 full token)
    #[arg(long, default_value = "1")]
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

pub async fn execute(args: &GetPriceArgs) -> Result<()> {
    let client = reqwest::Client::new();

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

    let input_decimals = resolve_decimals(&input_mint, &client).await?;
    let output_decimals = resolve_decimals(&output_mint, &client).await?;
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

    // Compute price ratio from quote data, normalizing raw amounts by token decimals
    let price_info = if let Some(data) = resp.get("data") {
        let raw_input: f64 = data["inputAmount"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(raw_amount as f64);
        let raw_output: f64 = data["outputAmount"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let price_impact_pct = data["priceImpactPct"].as_f64().unwrap_or(0.0);

        // Convert raw amounts to human-readable by dividing by 10^decimals
        let input_human = raw_input / 10f64.powi(input_decimals as i32);
        let output_human = raw_output / 10f64.powi(output_decimals as i32);

        let price = if input_human > 0.0 {
            output_human / input_human
        } else {
            0.0
        };
        serde_json::json!({
            "inputMint": input_mint,
            "outputMint": output_mint,
            "price": price,
            "priceImpactPct": price_impact_pct,
            "inputAmount": input_human,
            "outputAmount": output_human,
            "quote": data,
        })
    } else {
        resp.clone()
    };

    println!("{}", serde_json::to_string_pretty(&price_info)?);
    Ok(())
}
