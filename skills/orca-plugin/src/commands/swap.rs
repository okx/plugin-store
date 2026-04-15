use crate::api;
use crate::config::{
    DEFAULT_SLIPPAGE_BPS, PRICE_IMPACT_BLOCK_THRESHOLD, PRICE_IMPACT_WARN_THRESHOLD,
    SOL_DECIMALS, SOL_NATIVE_MINT, SOLANA_RPC_URL, WSOL_MINT,
};
use crate::onchainos;
use clap::Args;
use serde::Serialize;
use std::process::Command;

#[derive(Args, Debug)]
pub struct SwapArgs {
    /// Input token mint address (use native SOL: 11111111111111111111111111111111)
    #[arg(long)]
    pub from_token: String,

    /// Output token mint address
    #[arg(long)]
    pub to_token: String,

    /// Amount in human-readable units (e.g. 0.5 for 0.5 SOL, 10 for 10 USDC)
    #[arg(long)]
    pub amount: f64,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u64,

    /// Skip security scan of output token (not recommended)
    #[arg(long)]
    pub skip_security_check: bool,
}

#[derive(Serialize)]
struct SwapOutput {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solscan_url: Option<String>,
    from_token: String,
    to_token: String,
    amount: f64,
    amount_display: String,
    slippage_bps: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    estimated_price_impact_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pool_address: Option<String>,
}

pub async fn execute(args: &SwapArgs, dry_run: bool) -> anyhow::Result<()> {
    // ─── dry_run guard — wallet resolution must come AFTER this block ───
    if dry_run {
        let output = SwapOutput {
            ok: true,
            dry_run: Some(true),
            tx_hash: None,
            solscan_url: None,
            from_token: args.from_token.clone(),
            to_token: args.to_token.clone(),
            amount: args.amount,
            amount_display: format!("{:.2}", args.amount),
            slippage_bps: args.slippage_bps,
            estimated_price_impact_pct: None,
            warning: Some("dry_run=true — transaction not submitted".to_string()),
            error: None,
            pool_address: None,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // ─── Resolve wallet and pre-flight balance check ───
    let wallet = onchainos::resolve_wallet_solana()?;

    if args.from_token == SOL_NATIVE_MINT {
        let lamports = onchainos::get_sol_balance(&wallet, SOLANA_RPC_URL)
            .await
            .unwrap_or(0);
        let lamports_needed = (args.amount * 10u64.pow(SOL_DECIMALS) as f64) as u64;
        if lamports < lamports_needed {
            anyhow::bail!(
                "Insufficient SOL balance: need {:.6} SOL, have {:.6} SOL. \
                 Add funds to your wallet before swapping.",
                args.amount,
                lamports as f64 / 10u64.pow(SOL_DECIMALS) as f64,
            );
        }
    } else {
        let ui_balance = onchainos::get_spl_balance(&wallet, &args.from_token, SOLANA_RPC_URL)
            .await
            .unwrap_or(0.0);
        if ui_balance < args.amount {
            anyhow::bail!(
                "Insufficient token balance: need {:.6}, have {:.6} for mint {}. \
                 Add funds to your wallet before swapping.",
                args.amount,
                ui_balance,
                args.from_token,
            );
        }
    }

    // ─── Security scan of output token ───
    if !args.skip_security_check {
        let to_check = if args.to_token == SOL_NATIVE_MINT {
            WSOL_MINT
        } else {
            &args.to_token
        };
        match onchainos::security_token_scan(to_check) {
            Ok(risk) if risk == "block" => {
                let output = SwapOutput {
                    ok: false,
                    dry_run: None,
                    tx_hash: None,
                    solscan_url: None,
                    from_token: args.from_token.clone(),
                    to_token: args.to_token.clone(),
                    amount: args.amount,
                    amount_display: format!("{:.2}", args.amount),
                    slippage_bps: args.slippage_bps,
                    estimated_price_impact_pct: None,
                    warning: None,
                    error: Some(format!(
                        "Security scan blocked token {}: high-risk token, swap aborted",
                        args.to_token
                    )),
                    pool_address: None,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
                return Ok(());
            }
            Ok(risk) if risk == "warn" => {
                eprintln!(
                    "WARNING: Security scan returned 'warn' for token {}. Proceeding with caution.",
                    args.to_token
                );
            }
            _ => {}
        }
    }

    // ─── Fetch pool info for price impact estimation ───
    let client = reqwest::Client::new();
    let all_pools = api::fetch_all_pools(&client).await?;

    let normalize = |mint: &str| -> String {
        if mint == SOL_NATIVE_MINT {
            WSOL_MINT.to_string()
        } else {
            mint.to_string()
        }
    };
    let from_norm = normalize(&args.from_token);
    let to_norm = normalize(&args.to_token);

    let mut matching = api::filter_pools_by_pair(&all_pools, &from_norm, &to_norm);
    matching.sort_by(|a, b| {
        b.tvl
            .unwrap_or(0.0)
            .partial_cmp(&a.tvl.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let (best_pool_address, price_impact, pool_warning) = if let Some(pool) = matching.first() {
        let price = pool.price.unwrap_or(0.0);
        let tvl = pool.tvl.unwrap_or(1_000_000.0);
        let impact = api::estimate_price_impact(args.amount * price.max(1.0), tvl);
        let warn = if impact >= PRICE_IMPACT_WARN_THRESHOLD {
            Some(format!("Estimated price impact {:.2}%", impact))
        } else {
            None
        };
        (Some(pool.address.clone()), impact, warn)
    } else {
        eprintln!(
            "No pool found for pair {} / {} — proceeding with swap anyway (onchainos will route)",
            args.from_token, args.to_token
        );
        (None, 0.0, None)
    };

    // ─── Block if price impact is too high ───
    if price_impact >= PRICE_IMPACT_BLOCK_THRESHOLD {
        let output = SwapOutput {
            ok: false,
            dry_run: None,
            tx_hash: None,
            solscan_url: None,
            from_token: args.from_token.clone(),
            to_token: args.to_token.clone(),
            amount: args.amount,
            amount_display: format!("{:.2}", args.amount),
            slippage_bps: args.slippage_bps,
            estimated_price_impact_pct: Some(price_impact),
            warning: None,
            error: Some(format!(
                "Price impact {:.2}% exceeds block threshold of {}%. Swap aborted.",
                price_impact, PRICE_IMPACT_BLOCK_THRESHOLD
            )),
            pool_address: best_pool_address,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // ─── Execute swap via onchainos dex swap execute ───
    // Convert slippage from bps to percentage string
    let slippage_pct = format!("{:.4}", args.slippage_bps as f64 / 100.0);

    let result = execute_swap_onchainos(
        &wallet,
        &args.from_token,
        &args.to_token,
        args.amount,
        &slippage_pct,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let solscan_url = if !tx_hash.is_empty() && tx_hash != "pending" {
        Some(format!("https://solscan.io/tx/{}", tx_hash))
    } else {
        None
    };

    let output = SwapOutput {
        ok: result["ok"].as_bool().unwrap_or(false),
        dry_run: None,
        tx_hash: Some(tx_hash),
        solscan_url,
        from_token: args.from_token.clone(),
        to_token: args.to_token.clone(),
        amount: args.amount,
        amount_display: format!("{:.2}", args.amount),
        slippage_bps: args.slippage_bps,
        estimated_price_impact_pct: Some(price_impact),
        warning: pool_warning,
        error: if result["ok"].as_bool().unwrap_or(false) {
            None
        } else {
            result["error"]
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| result["message"].as_str().map(|s| s.to_string()))
        },
        pool_address: best_pool_address,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Execute swap via `onchainos swap execute`.
/// This is the primary path — onchainos handles routing, signing, and broadcasting.
async fn execute_swap_onchainos(
    wallet: &str,
    from_token: &str,
    to_token: &str,
    amount: f64,
    slippage_pct: &str,
) -> anyhow::Result<serde_json::Value> {
    let amount_str = amount.to_string();
    let output = Command::new("onchainos")
        .args([
            "swap",
            "execute",
            "--chain",
            "501",
            "--from",
            from_token,
            "--to",
            to_token,
            "--readable-amount",
            &amount_str,
            "--slippage",
            slippage_pct,
            "--wallet",
            &wallet,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if stdout.trim().is_empty() {
        anyhow::bail!(
            "onchainos swap execute returned empty output. stderr: {}",
            stderr
        );
    }

    serde_json::from_str(&stdout).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse onchainos output: {}. stdout: {}",
            e,
            stdout
        )
    })
}
