use clap::Args;
use serde_json::{json, Value};

use crate::api::{get_spot_meta, info_post};
use crate::config::info_url;

/// Buy USDH (Hyperliquid native stablecoin) for USDC on the spot market.
///
/// USDH is the collateral token for HIP-4 outcome contracts. The USDH/USDC
/// pair is essentially pegged 1:1 (mainnet ratio ~0.999995 as of 2026-05-05);
/// a small premium above peg is normal at touch.
///
/// Under the hood: places a limit-buy at `--max-price` on the USDH/USDC spot
/// pair. If the best ask is at or below `--max-price` the order fills
/// immediately at best ask (limit-price-protected market behavior). If the
/// peg is broken (best ask > max-price), the command bails with a structured
/// error rather than execute at a bad rate.
///
/// Examples:
///   # Acquire $5 of USDH at peg
///   hyperliquid-plugin usdh-fund --amount 5 --confirm
///
///   # Tighter peg tolerance
///   hyperliquid-plugin usdh-fund --amount 50 --max-price 1.0005 --confirm
///
///   # Preview only
///   hyperliquid-plugin usdh-fund --amount 5 --dry-run
#[derive(Args)]
pub struct UsdhFundArgs {
    /// Amount of USDH to acquire (in base units; e.g. `5` = 5 USDH).
    /// USDC required ≈ amount * best_ask. USDC must be in your spot account
    /// (run `transfer --from perp` if it lives in perp).
    #[arg(long)]
    pub amount: f64,

    /// Maximum acceptable USDH/USDC price. Defaults to 1.001 (0.1% premium
    /// above peg). Set lower to be stricter, higher to tolerate spread.
    /// The command refuses to submit if best ask exceeds this value.
    #[arg(long, default_value_t = 1.001)]
    pub max_price: f64,

    /// Show payload without signing or submitting.
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, shows a preview only).
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: UsdhFundArgs) -> anyhow::Result<()> {
    let info = info_url();

    if args.amount <= 0.0 {
        println!(
            "{}",
            super::error_response(
                &format!("--amount must be positive (got {})", args.amount),
                "INVALID_ARGUMENT",
                "Specify USDH amount, e.g. --amount 5",
            )
        );
        return Ok(());
    }
    if args.max_price <= 0.0 || args.max_price > 1.5 {
        println!(
            "{}",
            super::error_response(
                &format!("--max-price out of sanity range (got {})", args.max_price),
                "INVALID_ARGUMENT",
                "USDH is pegged to 1 USDC; --max-price typically 1.0005..1.005. Above 1.5 is rejected to prevent fat-finger.",
            )
        );
        return Ok(());
    }

    // Discover the USDH/USDC market index dynamically (mainnet @230, testnet @1338,
    // not hardcoded so the same binary works on both).
    let spot_meta = match get_spot_meta(info).await {
        Ok(v) => v,
        Err(e) => return print_api_err(&e, "spotMeta"),
    };
    let (mkt_idx, usdh_token_idx) = match find_usdh_market(&spot_meta) {
        Some(t) => t,
        None => {
            println!(
                "{}",
                super::error_response(
                    "USDH/USDC market not found in spotMeta",
                    "API_ERROR",
                    "USDH may not be deployed on this Hyperliquid environment. HIP-4 went live on mainnet 2026-05-02 with USDH at spot token index 360 / market @230.",
                )
            );
            return Ok(());
        }
    };
    let coin_str = format!("@{}", mkt_idx);

    // Probe l2Book best ask. If broken peg, refuse early.
    let book_resp = match info_post(info, json!({"type": "l2Book", "coin": coin_str.clone()})).await {
        Ok(v) => v,
        Err(e) => return print_api_err(&e, "l2Book"),
    };
    let best_ask: f64 = book_resp["levels"]
        .as_array()
        .and_then(|a| a.get(1)) // levels[1] = asks
        .and_then(|asks| asks.as_array())
        .and_then(|asks| asks.first())
        .and_then(|lvl| lvl["px"].as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    if best_ask <= 0.0 {
        println!(
            "{}",
            super::error_response(
                &format!("No asks available on USDH/USDC market ({})", coin_str),
                "NO_LIQUIDITY",
                "USDH spot book may be empty right now. Try again shortly, or fund a different way (deposit USDH to your wallet from elsewhere).",
            )
        );
        return Ok(());
    }
    if best_ask > args.max_price {
        println!(
            "{}",
            super::error_response(
                &format!(
                    "USDH peg deviation: best ask {} > max-price {}",
                    best_ask, args.max_price
                ),
                "PEG_DEVIATION",
                "Either raise --max-price tolerance, or wait for the peg to recover. USDH is normally ~1.000.",
            )
        );
        return Ok(());
    }

    // Use max_price as the limit (so HL fills at best_ask <= max_price immediately).
    // GTC keeps the order resting if best_ask gaps up between probe and fill.
    let limit_px = args.max_price;
    let estimated_usdc_cost = args.amount * best_ask;

    let preview = json!({
        "ok": true,
        "stage": if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" },
        "preview": {
            "action": "spot_buy_usdh",
            "market": coin_str,
            "usdh_token_index": usdh_token_idx,
            "amount_usdh": args.amount,
            "best_ask": best_ask,
            "max_price": args.max_price,
            "limit_price": limit_px,
            "estimated_usdc_cost": format!("{:.6}", estimated_usdc_cost),
            "tif": "Gtc",
            "note": "Submits as a GTC limit at max-price. Fills immediately at best_ask if liquidity covers the size; otherwise rests.",
        },
        "tip": "Once filled, use `outcome-buy` to deploy the USDH into HIP-4 outcomes.",
    });

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[DRY RUN] No order signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[PREVIEW] Add --confirm to submit the limit-buy.");
        return Ok(());
    }

    // Delegate to the existing spot_order flow rather than re-implementing
    // the signing / asset-id resolution. We construct the args struct manually
    // so we don't go through clap parsing.
    let spot_args = crate::commands::spot_order::SpotOrderArgs {
        coin: "USDH".to_string(),
        side: "buy".to_string(),
        size: format!("{}", args.amount),
        r#type: "limit".to_string(),
        price: Some(format!("{}", limit_px)),
        slippage: 5.0,
        post_only: false,
        dry_run: false,
        confirm: true,
    };
    crate::commands::spot_order::run(spot_args).await
}

/// Find the USDH/USDC market in spotMeta. Returns (market_index, usdh_token_index)
/// or None if USDH is not deployed.
fn find_usdh_market(spot_meta: &Value) -> Option<(u64, u64)> {
    let empty = vec![];
    let tokens = spot_meta["tokens"].as_array().unwrap_or(&empty);
    let universe = spot_meta["universe"].as_array().unwrap_or(&empty);

    // Locate USDH by name (preferred) — token index can vary across environments.
    let usdh_idx: u64 = tokens
        .iter()
        .find(|t| t["name"].as_str() == Some("USDH"))
        .and_then(|t| t["index"].as_u64())?;

    // USDC is always token index 0 on Hyperliquid.
    const USDC_TOKEN_IDX: u64 = 0;

    // Find universe entry whose tokens = [usdh_idx, 0] (base USDH, quote USDC).
    for m in universe {
        let arr = m["tokens"].as_array();
        let base = arr.and_then(|a| a.first()).and_then(|v| v.as_u64());
        let quote = arr.and_then(|a| a.get(1)).and_then(|v| v.as_u64());
        if base == Some(usdh_idx) && quote == Some(USDC_TOKEN_IDX) {
            let mkt_idx = m["index"].as_u64()?;
            return Some((mkt_idx, usdh_idx));
        }
    }
    None
}

fn print_api_err(e: &anyhow::Error, what: &str) -> anyhow::Result<()> {
    println!(
        "{}",
        super::error_response(
            &format!("{} fetch failed: {:#}", what, e),
            "API_ERROR",
            "Hyperliquid info endpoint may be limited; retry shortly.",
        )
    );
    Ok(())
}
