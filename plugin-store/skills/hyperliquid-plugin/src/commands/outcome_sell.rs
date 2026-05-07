use clap::Args;
use serde_json::{json, Value};

use crate::api::{
    fetch_outcome_meta, get_spot_clearinghouse_state, outcome_asset_id, outcome_balance_coin,
    outcome_trade_coin, OutcomeSpec,
};
use crate::config::{exchange_url, info_url, now_ms, ARBITRUM_CHAIN_ID, CHAIN_ID};
use crate::onchainos::{onchainos_hl_sign, report_plugin_info, resolve_wallet};
use crate::signing::{build_limit_order_action, submit_exchange_request};

/// Sell a HIP-4 outcome side (YES or NO). Closes a long position OR opens a
/// short on that side.
///
/// HIP-4 does not have classical "reduce-only" semantics: a sell on a YES leg
/// from a flat account opens a short YES, which is economically equivalent to
/// long NO. The matching engine classifies each fill as MINT / NORMAL TRADE /
/// BURN automatically based on counterparty inventory — the user just submits
/// a normal sell order.
///
/// Examples:
///   # Close 5 YES shares of outcome 2 at $0.70 (assuming you hold ≥ 5 long YES)
///   hyperliquid-plugin outcome-sell --outcome 2 --side yes --shares 5 --price 0.70 --confirm
///
///   # Aggressive sell (IOC), accepts any price down to 0.001
///   hyperliquid-plugin outcome-sell --outcome 2 --side yes --shares 5 --price 0.001 --tif Ioc --confirm
///
///   # Sell from semantic id
///   hyperliquid-plugin outcome-sell --outcome BTC-79980-1d --side no --shares 10 --price 0.25 --confirm
#[derive(Args)]
pub struct OutcomeSellArgs {
    /// Outcome identifier — numeric (e.g. `2`) or recurring semantic id
    /// (e.g. `BTC-79980-1d`).
    #[arg(long)]
    pub outcome: String,

    /// Which leg of the binary outcome: `yes` (side 0) or `no` (side 1).
    #[arg(long, value_parser = ["yes", "no"])]
    pub side: String,

    /// Number of shares to sell.
    #[arg(long)]
    pub shares: f64,

    /// Limit price in USDH per share. Must be in 0.001..0.999.
    /// For aggressive market-like fill, use 0.001 with `--tif Ioc`.
    #[arg(long)]
    pub price: f64,

    /// Time-in-force: `Gtc` (default; resting limit), `Ioc` (immediate-or-cancel),
    /// `Alo` (add-liquidity-only / post-only).
    #[arg(long, default_value = "Gtc", value_parser = ["Gtc", "Ioc", "Alo"])]
    pub tif: String,

    /// Allow opening a short on this side (selling more than current holding,
    /// or selling from flat). Without this flag, the command refuses to submit
    /// if --shares exceeds your current long balance — preventing accidental
    /// short positions that have unbounded loss exposure (capped at -1 USDH/share
    /// for outcomes, but still confusing for new users).
    #[arg(long)]
    pub allow_short: bool,

    /// Skip the position size pre-flight check (useful for orders not relying
    /// on current spot state, e.g. closing immediately after a fill).
    #[arg(long)]
    pub skip_position_check: bool,

    /// Optional strategy ID tag for attribution. All filled/resting outcome sells
    /// are reported to the OKX backend regardless; this flag just attaches a
    /// strategy label. Empty if omitted.
    #[arg(long)]
    pub strategy_id: Option<String>,

    /// Show payload without signing or submitting.
    #[arg(long)]
    pub dry_run: bool,

    /// Confirm and submit (without this flag, prints a preview only).
    #[arg(long)]
    pub confirm: bool,
}

pub async fn run(args: OutcomeSellArgs) -> anyhow::Result<()> {
    let info = info_url();
    let exchange = exchange_url();

    // ─── Validate inputs ─────────────────────────────────────────────────────
    if args.shares <= 0.0 {
        return print_invalid_arg(&format!("--shares must be positive (got {})", args.shares));
    }
    if !(0.001..=0.999).contains(&args.price) {
        return print_invalid_arg(&format!(
            "--price {} out of HIP-4 range [0.001..0.999]. Outcome prices represent implied probability.",
            args.price
        ));
    }
    let side: u8 = if args.side.eq_ignore_ascii_case("yes") { 0 } else { 1 };

    // ─── Resolve outcome_id (numeric or semantic) ─────────────────────────────
    let outcomes = match fetch_outcome_meta(info).await {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("outcomeMeta fetch failed: {:#}", e),
                    "API_ERROR",
                    "Hyperliquid info endpoint may be limited; retry shortly.",
                )
            );
            return Ok(());
        }
    };
    let (outcome_id, matched) = match resolve_outcome(&args.outcome, &outcomes) {
        Some(v) => v,
        None => {
            let known: Vec<String> = outcomes
                .iter()
                .map(|o| {
                    o.parse_recurring()
                        .map(|r| {
                            format!(
                                "{} (id={}, semantic={}-{:.0}-{})",
                                o.name, o.outcome_id, r.underlying, r.target_price, r.period
                            )
                        })
                        .unwrap_or_else(|| format!("{} (id={})", o.name, o.outcome_id))
                })
                .collect();
            println!(
                "{}",
                super::error_response(
                    &format!("Outcome '{}' not found in outcomeMeta", args.outcome),
                    "OUTCOME_NOT_FOUND",
                    &format!("Known outcomes: {}", known.join(" | ")),
                )
            );
            return Ok(());
        }
    };

    let trade_coin = outcome_trade_coin(outcome_id, side);
    let balance_coin = outcome_balance_coin(outcome_id, side);
    let asset_id = outcome_asset_id(outcome_id, side) as usize;

    let side_name = if side == 0 {
        matched.side_names.0.clone()
    } else {
        matched.side_names.1.clone()
    };

    // ─── Resolve wallet ──────────────────────────────────────────────────────
    let wallet = match resolve_wallet(CHAIN_ID) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run `onchainos wallet addresses` to verify login.",
                )
            );
            return Ok(());
        }
    };

    // ─── Pre-flight: check current position covers the sell ──────────────────
    let current_position: Option<f64> = if args.skip_position_check {
        None
    } else {
        match get_spot_clearinghouse_state(info, &wallet).await {
            Ok(state) => {
                let empty = vec![];
                let pos = state["balances"].as_array().unwrap_or(&empty)
                    .iter()
                    .find(|b| b["coin"].as_str() == Some(balance_coin.as_str()))
                    .and_then(|b| b["total"].as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                Some(pos)
            }
            Err(_) => None,
        }
    };
    if let Some(pos) = current_position {
        if pos < args.shares && !args.allow_short {
            let shortfall = args.shares - pos;
            let suggestion = if pos == 0.0 {
                format!(
                    "No long position on {}. Either: (a) open a long on the OTHER leg (`outcome-buy --side {}`), or (b) pass --allow-short to deliberately open a short on this leg.",
                    balance_coin,
                    if side == 0 { "no" } else { "yes" }
                )
            } else {
                format!(
                    "You hold {:.4} long {} on {}. Reduce --shares to ≤ {:.4}, or pass --allow-short to open a short for the {:.4} excess.",
                    pos, side_name, balance_coin, pos, shortfall
                )
            };
            println!(
                "{}",
                super::error_response(
                    &format!(
                        "Sell exceeds current long position: have {:.4}, want to sell {:.4} (would short {:.4})",
                        pos, args.shares, shortfall
                    ),
                    "WOULD_OPEN_SHORT",
                    &suggestion,
                )
            );
            return Ok(());
        }
    }

    // ─── Build action ────────────────────────────────────────────────────────
    let price_str = format!("{}", args.price);
    let shares_str = format!("{}", args.shares);
    let action = build_limit_order_action(
        asset_id,
        false, // is_buy = false (sell)
        &price_str,
        &shares_str,
        false, // reduce_only — outcome positions don't gate on this
        &args.tif,
    );
    let nonce = now_ms();

    // ─── Preview ─────────────────────────────────────────────────────────────
    let proceeds = args.shares * args.price;
    let preview = json!({
        "ok": true,
        "stage": if args.dry_run { "dry_run" } else if args.confirm { "submit" } else { "preview" },
        "preview": {
            "action": "outcome_sell",
            "outcome_id": outcome_id,
            "outcome_name": matched.name,
            "outcome_description": matched.description,
            "side": args.side.to_lowercase(),
            "side_name": side_name,
            "trade_coin": trade_coin,
            "balance_coin": balance_coin,
            "asset_id": asset_id,
            "shares": args.shares,
            "limit_price": args.price,
            "tif": args.tif,
            "estimated_usdh_proceeds": format!("{:.4}", proceeds),
            "current_position": current_position.map(|p| format!("{:.4}", p)),
            "would_short": current_position.map(|p| if args.shares > p { args.shares - p } else { 0.0 })
                                            .map(|s| format!("{:.4}", s)),
            "allow_short": args.allow_short,
            "nonce": nonce,
        },
    });

    if args.dry_run {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!("[DRY RUN] No order signed or submitted.");
        return Ok(());
    }
    if !args.confirm {
        println!("{}", serde_json::to_string_pretty(&preview)?);
        eprintln!(
            "[PREVIEW] Add --confirm to sign and submit. Estimated proceeds: {:.4} USDH.",
            proceeds
        );
        return Ok(());
    }

    // ─── Sign & submit ───────────────────────────────────────────────────────
    let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "SIGNING_FAILED",
                    "Retry the command. If the issue persists, check `onchainos wallet status`.",
                )
            );
            return Ok(());
        }
    };
    eprintln!("[outcome-sell] Submitting to Hyperliquid exchange...");
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "TX_SUBMIT_FAILED",
                    "Retry the command. If persistent, check the order book and wallet state.",
                )
            );
            return Ok(());
        }
    };

    // ─── Inspect response ────────────────────────────────────────────────────
    let status = result["status"].as_str().unwrap_or("");
    if status != "ok" {
        println!(
            "{}",
            super::error_response(
                &format!("Hyperliquid rejected outcome-sell: {}", serde_json::to_string(&result).unwrap_or_default()),
                "TX_REJECTED",
                "Check `result.response` for HL's specific reason. Common: market closed, price out of range, position smaller than --shares at order time.",
            )
        );
        return Ok(());
    }
    let statuses = result["response"]["data"]["statuses"]
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(Value::Null);
    let avg_px = statuses["filled"]["avgPx"].as_str().map(|s| s.to_string());
    let oid = statuses["filled"]["oid"]
        .as_u64()
        .or_else(|| statuses["resting"]["oid"].as_u64());

    // Attribution: report every outcome-sell that produced an oid (filled or resting).
    // strategy_id is optional — empty string when not provided so backend still
    // receives a record. Same shape as outcome-buy reporting (USDH symbol, market_id
    // is the trade coin form #N).
    if let Some(oid_val) = oid {
        let ts_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let sid = args.strategy_id.as_deref().unwrap_or("");
        let report_payload = serde_json::json!({
            "wallet": wallet,
            "proxyAddress": "",
            "order_id": oid_val.to_string(),
            "tx_hashes": [],
            "market_id": trade_coin,
            "asset_id": format!("{}", asset_id),
            "side": "SELL",
            "amount": shares_str,
            "symbol": "USDH",
            "price": avg_px.clone().unwrap_or_else(|| price_str.clone()),
            "timestamp": ts_now,
            "strategy_id": sid,
            "plugin_name": "hyperliquid-plugin",
        });
        if let Err(e) = report_plugin_info(&report_payload) {
            eprintln!("[hyperliquid] Warning: report-plugin-info failed: {}", e);
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "action": "outcome_sell",
            "outcome_id": outcome_id,
            "outcome_name": matched.name,
            "side": args.side.to_lowercase(),
            "trade_coin": trade_coin,
            "balance_coin": balance_coin,
            "shares": args.shares,
            "limit_price": args.price,
            "filled_avg_price": avg_px,
            "filled": statuses["filled"].is_object(),
            "resting": statuses["resting"].is_object(),
            "order_id": oid,
            "estimated_usdh_proceeds_at_limit": format!("{:.4}", proceeds),
            "result": result,
            "tip": "Run `outcome-positions` to verify the new size.",
        }))?
    );
    Ok(())
}

/// Resolve `--outcome` arg to (outcome_id, &OutcomeSpec). Same logic as outcome-buy.
fn resolve_outcome<'a>(
    arg: &str,
    outcomes: &'a [OutcomeSpec],
) -> Option<(u32, &'a OutcomeSpec)> {
    if let Ok(id) = arg.parse::<u32>() {
        if let Some(o) = outcomes.iter().find(|o| o.outcome_id == id) {
            return Some((id, o));
        }
    }
    let arg_upper = arg.to_uppercase();
    for o in outcomes {
        if let Some(r) = o.parse_recurring() {
            let candidates = [
                format!("{}-{:.0}-{}", r.underlying, r.target_price, r.period).to_uppercase(),
                format!("{}-{}-{}", r.underlying, r.target_price as u64, r.period).to_uppercase(),
            ];
            if candidates.iter().any(|c| c == &arg_upper) {
                return Some((o.outcome_id, o));
            }
        }
    }
    None
}

fn print_invalid_arg(msg: &str) -> anyhow::Result<()> {
    println!(
        "{}",
        super::error_response(msg, "INVALID_ARGUMENT", "See `outcome-sell --help` for parameter details.")
    );
    Ok(())
}
