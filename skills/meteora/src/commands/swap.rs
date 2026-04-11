use clap::Args;
use crate::onchainos;
use crate::config::DEFAULT_SLIPPAGE_PCT;

#[derive(Args, Debug)]
pub struct SwapArgs {
    /// Source token mint address (or 11111111111111111111111111111111 for native SOL)
    #[arg(long)]
    pub from_token: String,

    /// Destination token mint address
    #[arg(long)]
    pub to_token: String,

    /// Human-readable input amount (e.g. "1.5" for 1.5 SOL)
    #[arg(long)]
    pub amount: String,

    /// Slippage tolerance in percent (e.g. "0.5" for 0.5%). Defaults to auto-slippage.
    #[arg(long)]
    pub slippage: Option<f64>,

    /// Wallet address (Solana pubkey). If omitted, uses the currently logged-in wallet.
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(args: &SwapArgs, dry_run: bool) -> anyhow::Result<()> {
    // dry_run: show quote instead of executing swap
    if dry_run {
        let raw = onchainos::dex_quote_solana(
            &args.from_token,
            &args.to_token,
            &args.amount,
        )?;

        // Extract meaningful fields from data[0]
        let data0 = &raw["data"][0];
        let out_amount_raw = data0["toTokenAmount"]
            .as_str()
            .or_else(|| data0["outAmount"].as_str())
            .unwrap_or("unknown");
        let to_decimals = data0["toToken"]["decimal"]
            .as_str()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(6);
        let to_symbol = data0["toToken"]["tokenSymbol"].as_str().unwrap_or("unknown");
        let from_symbol = data0["fromToken"]["tokenSymbol"].as_str().unwrap_or("unknown");
        let price_impact: f64 = data0["priceImpactPercent"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .map(f64::abs)
            .unwrap_or(0.0);
        let to_amount_readable = out_amount_raw
            .parse::<u128>()
            .ok()
            .map(|r| format!("{:.6}", r as f64 / 10f64.powi(to_decimals as i32)))
            .unwrap_or_else(|| "unknown".to_string());

        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "message": "Dry run: showing quote only. No transaction submitted.",
            "from_token": args.from_token,
            "from_symbol": from_symbol,
            "to_token": args.to_token,
            "to_symbol": to_symbol,
            "amount": args.amount,
            "estimated_output": to_amount_readable,
            "estimated_output_raw": out_amount_raw,
            "price_impact_pct": price_impact,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Resolve wallet address AFTER dry_run guard
    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!("Cannot resolve wallet address. Pass --wallet <address> or log in via onchainos.\nError: {e}")
        })?
    };

    if wallet.is_empty() {
        anyhow::bail!("Wallet address is empty. Pass --wallet <address> or log in via onchainos.");
    }

    // Build slippage string
    let slippage_str = args
        .slippage
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_SLIPPAGE_PCT.to_string());

    // Execute swap via onchainos swap execute
    // NOTE: Solana does NOT need --force flag
    let result = onchainos::dex_swap_execute_solana(
        &args.from_token,
        &args.to_token,
        &args.amount,
        &wallet,
        Some(&slippage_str),
    )?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let ok = result["ok"].as_bool().unwrap_or(false);

    let output = serde_json::json!({
        "ok": ok,
        "from_token": args.from_token,
        "to_token": args.to_token,
        "amount": args.amount,
        "wallet": wallet,
        "tx_hash": tx_hash,
        "explorer_url": if tx_hash != "pending" && !tx_hash.is_empty() {
            format!("https://solscan.io/tx/{}", tx_hash)
        } else {
            String::new()
        },
        "raw_result": result,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
