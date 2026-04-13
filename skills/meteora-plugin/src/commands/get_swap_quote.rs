use clap::Args;
use std::process::Command;
use crate::config::PRICE_IMPACT_WARN_THRESHOLD;

#[derive(Args, Debug)]
pub struct GetSwapQuoteArgs {
    /// Source token mint address (or 11111111111111111111111111111111 for native SOL)
    #[arg(long)]
    pub from_token: String,

    /// Destination token mint address
    #[arg(long)]
    pub to_token: String,

    /// Human-readable input amount (e.g. "1.5" for 1.5 SOL)
    #[arg(long)]
    pub amount: String,
}

pub async fn execute(args: &GetSwapQuoteArgs) -> anyhow::Result<()> {
    // Use onchainos swap quote for Solana
    let output = Command::new("onchainos")
        .args([
            "swap", "quote",
            "--chain", "solana",
            "--from", &args.from_token,
            "--to", &args.to_token,
            "--readable-amount", &args.amount,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let raw: serde_json::Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({
        "raw_stdout": stdout.to_string(),
        "raw_stderr": stderr.to_string(),
    }));

    // onchainos swap quote returns { "data": [ { ... } ], "ok": true }
    // Extract the first element of the data array
    let data0 = &raw["data"][0];

    // Price impact: key is "priceImpactPercent" (a string like "-0.01"), not "priceImpactPercentage"
    let price_impact = data0["priceImpactPercent"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .map(f64::abs) // negative means positive impact; use abs for comparison
        .or_else(|| data0["priceImpactPercentage"].as_f64())
        .unwrap_or(0.0);

    let price_impact_warn = price_impact > PRICE_IMPACT_WARN_THRESHOLD;

    // toTokenAmount and fromTokenAmount are at data[0], not data directly
    let out_amount_raw = data0["toTokenAmount"]
        .as_str()
        .or_else(|| data0["outAmount"].as_str())
        .unwrap_or("unknown");

    let from_amount_raw = data0["fromTokenAmount"]
        .as_str()
        .or_else(|| data0["inAmount"].as_str())
        .unwrap_or(&args.amount);

    // Compute human-readable output amount using toToken decimals
    let to_decimals = data0["toToken"]["decimal"]
        .as_str()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(6);
    let to_symbol = data0["toToken"]["tokenSymbol"].as_str().unwrap_or("unknown");
    let from_symbol = data0["fromToken"]["tokenSymbol"].as_str().unwrap_or("unknown");

    let to_amount_readable = out_amount_raw
        .parse::<u128>()
        .ok()
        .map(|raw| format!("{:.6}", raw as f64 / 10f64.powi(to_decimals as i32)))
        .unwrap_or_else(|| "unknown".to_string());

    let result = serde_json::json!({
        "ok": true,
        "quote": {
            "from_token": args.from_token,
            "from_symbol": from_symbol,
            "to_token": args.to_token,
            "to_symbol": to_symbol,
            "from_amount_readable": args.amount,
            "from_amount_raw": from_amount_raw,
            "to_amount_readable": to_amount_readable,
            "to_amount_raw": out_amount_raw,
            "price_impact_pct": price_impact,
            "price_impact_warning": if price_impact_warn {
                Some(format!("High price impact: {:.2}%. Consider splitting your trade.", price_impact))
            } else {
                None
            },
        },
        "raw_quote": raw,
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
