use clap::Args;
use crate::api::{
    fetch_perp_dexs, get_all_mids_for_dex, get_asset_meta_with_flags,
    get_clearinghouse_state_for_dex, get_spot_clearinghouse_state,
    parse_coin,
};
use crate::config::{info_url, exchange_url, normalize_coin, now_ms, CHAIN_ID, ARBITRUM_CHAIN_ID, USDC_ARBITRUM};
use crate::onchainos::{onchainos_hl_sign, report_plugin_info, resolve_wallet};
use crate::rpc::{arbitrum_rpc, erc20_balance};
use crate::signing::{
    build_bracketed_order_action, build_limit_order_action, build_market_order_action,
    build_update_leverage_action,
    round_px, submit_exchange_request,
};

#[derive(Args)]
pub struct OrderArgs {
    /// Coin to trade (e.g. BTC, ETH, SOL, ARB)
    #[arg(long)]
    pub coin: String,

    /// Side: buy (long) or sell (short)
    #[arg(long, value_parser = ["buy", "sell"])]
    pub side: String,

    /// Position size in base units (e.g. 0.01 for 0.01 BTC)
    #[arg(long)]
    pub size: String,

    /// Order type: market or limit
    #[arg(long, value_parser = ["market", "limit"], default_value = "market")]
    pub r#type: String,

    /// Limit price (required for limit orders)
    #[arg(long)]
    pub price: Option<String>,

    /// Stop-loss trigger price — attaches a stop-loss child order (bracket)
    #[arg(long)]
    pub sl_px: Option<f64>,

    /// Take-profit trigger price — attaches a take-profit child order (bracket)
    #[arg(long)]
    pub tp_px: Option<f64>,

    /// Leverage multiplier before placing (e.g. 10 for 10x cross). Sets account leverage for this
    /// coin first, then places the order. Omit to keep the current account setting.
    #[arg(long)]
    pub leverage: Option<u32>,

    /// Use isolated margin mode when --leverage is set (default is cross margin)
    #[arg(long)]
    pub isolated: bool,

    /// Reduce only — only reduce an existing position, never increase it
    #[arg(long)]
    pub reduce_only: bool,

    /// Dry run — preview order payload without signing or submitting
    #[arg(long)]
    pub dry_run: bool,

    /// Slippage tolerance for market orders, in percent (default 5.0 = 5%)
    /// The worst-fill price is mid × (1 ± slippage/100)
    #[arg(long, default_value = "5.0")]
    pub slippage: f64,

    /// Worst-fill slippage when a TP/SL bracket trigger fires, in percent (default 10.0 = 10%).
    /// Only applies when --sl-px or --tp-px is set
    #[arg(long, default_value = "10.0")]
    pub trigger_slippage: f64,

    /// Confirm and submit the order (without this flag, prints a preview)
    #[arg(long)]
    pub confirm: bool,

    /// Optional strategy ID tag for attribution. All orders are reported to the OKX
    /// backend regardless; this flag just attaches a strategy label. Empty if omitted.
    #[arg(long)]
    pub strategy_id: Option<String>,

    /// Autotrade (copy-trading) job ID from an OnchainOS execution card. Gates the
    /// order behind `onchainos agent autotrade-grant-check` (fail-closed) before any
    /// signing or submission. Skipped with --dry-run.
    #[arg(long)]
    pub autotrade_job: Option<String>,
}

/// Format a size value to exactly `decimals` decimal places, trimming trailing zeros.
fn fmt_size(sz: f64, decimals: u32) -> String {
    if decimals == 0 {
        format!("{:.0}", sz)
    } else {
        let s = format!("{:.prec$}", sz, prec = decimals as usize);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

pub async fn run(args: OrderArgs) -> anyhow::Result<()> {
    let is_autotrade = args.autotrade_job.is_some();
    if let Some(job_id) = args.autotrade_job.as_deref() {
        if let Err(rejection) = super::check_autotrade_job_id(job_id) {
            println!("{}", rejection);
            return Ok(());
        }
    }

    let info = info_url();
    let exchange = exchange_url();
    // HIP-3: detect dex prefix from coin name (e.g. "xyz:CL" -> dex="xyz", base="CL").
    // Coin lookup downstream happens via get_asset_meta_for_coin which understands prefixes.
    // For position match (post-order), the coin field on a HIP-3 position is the FULL prefixed
    // name (e.g. "xyz:CL"), so we keep the raw user input for matching.
    let (dex_opt, _base) = parse_coin(&args.coin);
    let coin = if dex_opt.is_some() {
        // Builder dex coins are case-sensitive on the dex prefix; normalize only the symbol.
        let (d, b) = parse_coin(&args.coin);
        format!("{}:{}", d.unwrap(), b.to_uppercase())
    } else {
        normalize_coin(&args.coin)
    };
    let is_buy = args.side.to_lowercase() == "buy";
    let nonce = now_ms();

    // Validate size is a number
    let size_f: f64 = match args.size.parse() {
        Ok(v) => v,
        Err(_) => {
            println!("{}", super::execution_error_response(
                &format!("Invalid size '{}' — must be a number (e.g. 0.01)", args.size),
                "INVALID_ARGUMENT",
                "Provide a numeric size value, e.g. --size 0.01",
                is_autotrade,
            ));
            return Ok(());
        }
    };

    if is_autotrade && (!args.slippage.is_finite() || args.slippage < 0.0 || args.slippage >= 100.0)
    {
        println!(
            "{}",
            super::execution_error_response(
                "--slippage must be a finite percentage in [0, 100) for autotrade",
                "INVALID_INPUT",
                "Provide a valid slippage percentage.",
                true,
            )
        );
        return Ok(());
    }
    if is_autotrade
        && (args.sl_px.is_some() || args.tp_px.is_some())
        && (!args.trigger_slippage.is_finite()
            || args.trigger_slippage < 0.0
            || args.trigger_slippage >= 100.0)
    {
        println!(
            "{}",
            super::execution_error_response(
                "--trigger-slippage must be a finite percentage in [0, 100) for autotrade",
                "INVALID_INPUT",
                "Provide a valid trigger slippage percentage.",
                true,
            )
        );
        return Ok(());
    }
    if is_autotrade
        && args
            .sl_px
            .into_iter()
            .chain(args.tp_px)
            .any(|price| !price.is_finite() || price <= 0.0)
    {
        println!(
            "{}",
            super::execution_error_response(
                "Autotrade SL/TP prices must be finite and positive.",
                "INVALID_INPUT",
                "Provide valid SL/TP prices.",
                true,
            )
        );
        return Ok(());
    }

    // Validate leverage range (Hyperliquid accepts 1–100)
    if let Some(lev) = args.leverage {
        if !(1..=100).contains(&lev) {
            println!("{}", super::execution_error_response(
                &format!("--leverage must be between 1 and 100 (got {})", lev),
                "INVALID_ARGUMENT",
                "Provide a leverage value between 1 and 100, e.g. --leverage 10",
                is_autotrade,
            ));
            return Ok(());
        }
    }

    // TP/SL bracket validation
    if let Some(sl) = args.sl_px {
        if is_buy && args.tp_px.is_some_and(|tp| tp <= sl) {
            println!("{}", super::execution_error_response(
                "Take-profit must be above stop-loss for a long position",
                "INVALID_ARGUMENT",
                "For a long: SL below entry, TP above entry.",
                is_autotrade,
            ));
            return Ok(());
        }
        if !is_buy && args.tp_px.is_some_and(|tp| tp >= sl) {
            println!("{}", super::execution_error_response(
                "Take-profit must be below stop-loss for a short position",
                "INVALID_ARGUMENT",
                "For a short: SL above entry, TP below entry.",
                is_autotrade,
            ));
            return Ok(());
        }
    }

    // ─── Fetch dex registry + meta + prices concurrently ─────────────────────
    let (registry_res, mids_res) = tokio::join!(
        fetch_perp_dexs(info),
        get_all_mids_for_dex(info, dex_opt.as_deref()),
    );
    let registry = registry_res.unwrap_or_default();
    let (asset_idx, sz_decimals, only_isolated) = match get_asset_meta_with_flags(info, &coin, &registry).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::execution_error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry. If using a builder DEX coin (e.g. xyz:CL), run `hyperliquid-plugin dex-list` to verify the DEX exists.", is_autotrade));
            return Ok(());
        }
    };

    let autotrade_size = if is_autotrade {
        match super::normalize_autotrade_size(&args.size, sz_decimals) {
            Some(size) => Some(size),
            None => {
                println!(
                    "{}",
                    super::execution_error_response(
                        &format!(
                            "Autotrade size '{}' must be a positive fixed-point decimal already aligned to {} decimal places.",
                            args.size, sz_decimals
                        ),
                        "INVALID_INPUT",
                        "Provide a positive size aligned to the market size grid.",
                        true,
                    )
                );
                return Ok(());
            }
        }
    } else {
        None
    };

    if is_autotrade {
        if args.r#type == "limit" {
            let Some(price) = args.price.as_deref() else {
                println!(
                    "{}",
                    super::execution_error_response(
                        "--price is required for limit orders",
                        "INVALID_INPUT",
                        "Provide a limit price.",
                        true,
                    )
                );
                return Ok(());
            };
            let parsed_price = price.parse::<f64>().ok();
            let aligned = parsed_price
                .filter(|value| value.is_finite() && *value > 0.0)
                .map(|value| round_px(value, sz_decimals))
                .is_some_and(|rounded| super::positive_decimals_equal(price, &rounded));
            if !aligned {
                println!(
                    "{}",
                    super::execution_error_response(
                        &format!(
                            "Autotrade limit price '{}' must already satisfy Hyperliquid's price precision.",
                            price
                        ),
                        "INVALID_INPUT",
                        "Provide an exchange-aligned limit price.",
                        true,
                    )
                );
                return Ok(());
            }
        }

        for (name, price) in [("--sl-px", args.sl_px), ("--tp-px", args.tp_px)] {
            if let Some(price) = price {
                let input = price.to_string();
                let rounded = round_px(price, sz_decimals);
                if !super::positive_decimals_equal(&input, &rounded) {
                    println!(
                        "{}",
                        super::execution_error_response(
                            &format!(
                                "Autotrade {} value '{}' must already satisfy Hyperliquid's price precision.",
                                name, input
                            ),
                            "INVALID_INPUT",
                            "Provide an exchange-aligned trigger price.",
                            true,
                        )
                    );
                    return Ok(());
                }
            }
        }
    }

    // HIP-3: Some RWA / equity markets (xyz:CL / xyz:HOOD / xyz:INTC / xyz:PLTR / xyz:COIN /
    // etc.) have `onlyIsolated: true` — HL rejects cross-margin orders on these with
    // "Cross margin is not allowed for this asset". Auto-promote to --isolated when this
    // flag is set, so the user doesn't need to memorize per-coin margin restrictions.
    let auto_isolated = only_isolated && !args.isolated;
    if auto_isolated {
        if is_autotrade && args.leverage.is_some() {
            println!(
                "{}",
                super::execution_error_response(
                    &format!(
                        "{} requires isolated margin, but the execution card did not include --isolated.",
                        coin
                    ),
                    "INVALID_INPUT",
                    "Add --isolated and retry.",
                    true,
                )
            );
            return Ok(());
        }
        eprintln!("[order] {} requires isolated margin (onlyIsolated=true) — auto-enabling --isolated.", coin);
    }
    let use_isolated = args.isolated || only_isolated;
    let mids = match mids_res {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::execution_error_response(&format!("{:#}", e), "API_ERROR", "Check your connection and retry.", is_autotrade));
            return Ok(());
        }
    };

    let current_price = mids
        .get(&coin)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let mid_f = current_price.parse::<f64>().unwrap_or(0.0);

    // ─── Size: round to szDecimals, then auto-bump if notional < $10 ─────────
    //
    // HL enforces a $10 minimum notional on every perp/spot order; broadcasts
    // below this revert with `Order must have minimum value of 10 USDC`.
    // Previous logic bumped the size by exactly one tick which often still
    // landed below $10 (e.g. NVDA @217.5 sz_decimals=3: 0.010→0.011 = $2.39).
    // Compute the smallest size such that size*mid >= $10 directly.
    let sz_factor = 10_f64.powi(sz_decimals as i32);
    let mut size_rounded = match autotrade_size.as_deref() {
        Some(size) => size.parse::<f64>().unwrap_or(0.0),
        None => (size_f * sz_factor).round() / sz_factor,
    };

    let notional_price = notional_price(&args.r#type, args.price.as_deref(), mid_f);

    if notional_price > 0.0 {
        let n = size_rounded * notional_price;
        if n > 0.0 && n < 10.0 {
            // On the autotrade path the amount submitted for authorization must equal the
            // amount broadcast. Bumping it up would let an order clear a grant for X and
            // then execute at more than X, so the shortfall is refused instead.
            if is_autotrade {
                println!(
                    "{}",
                    super::execution_error_response(
                        &format!(
                            "Order notional ${:.2} is below the $10 exchange minimum; size cannot be raised on the autotrade path.",
                            n
                        ),
                        "ORDER_BELOW_MIN_NOTIONAL",
                        "Increase the order size to the exchange minimum.",
                        true,
                    )
                );
                return Ok(());
            }
            // ceil(10 / mid * sz_factor) / sz_factor → smallest grid-aligned size with notional >= $10.
            // Add small epsilon to mid for floating-point safety so we don't land
            // exactly on $9.999999 due to rounding.
            let min_size = min_grid_size(MIN_ORDER_USD, notional_price, sz_decimals);
            eprintln!(
                "[auto-adjust] size {} → {} to meet $10 minimum notional (${:.2} → ${:.2})",
                fmt_size(size_rounded, sz_decimals),
                fmt_size(min_size, sz_decimals),
                n,
                min_size * notional_price,
            );
            size_rounded = min_size;
        }
    }
    let size_str = autotrade_size.unwrap_or_else(|| fmt_size(size_rounded, sz_decimals));
    // Value the order at the price it would actually execute at: the limit price
    // for a limit order, the mid for a market order. Reporting a mid-based
    // notional for a limit order priced away from the market understates what
    // the position and its margin will be if it fills.
    let notional = size_rounded * notional_price;

    // Slippage-protected price for market orders
    let slippage_multiplier = if is_buy { 1.0 + args.slippage / 100.0 } else { 1.0 - args.slippage / 100.0 };
    let slippage_px_str = round_px(mid_f * slippage_multiplier, sz_decimals);

    // ─── SL/TP prices rounded to correct precision ────────────────────────────
    let sl_px_str = args.sl_px.map(|px| round_px(px, sz_decimals));
    let tp_px_str = args.tp_px.map(|px| round_px(px, sz_decimals));

    // ─── Balance pre-flight (non-fatal — skip if wallet not connected) ────────
    // Shows Perp + Spot + Arbitrum. HyperEVM excluded per user preference.
    let wallet_opt = resolve_wallet(CHAIN_ID).ok();
    let arb_wallet_opt = resolve_wallet(ARBITRUM_CHAIN_ID).ok();

    struct Balances {
        perp: f64,
        spot: f64,
        arb: f64,
        /// HL account margin mode, from `userAbstraction` (e.g. "unifiedAccount").
        mode: String,
    }

    impl Balances {
        /// True when HL shares one margin pool, so spot USDC backs perp orders.
        fn unified(&self) -> bool {
            let m = self.mode.to_ascii_lowercase();
            m.contains("unified") || m.contains("portfolio")
        }

        /// Margin that can actually back a perp order right now.
        ///
        /// Under HL's unified / portfolio modes spot USDC backs perp orders
        /// directly and HL *rejects* spot->perp transfers ("Action disabled when
        /// unified account is active"). Counting only the perp clearinghouse
        /// balance there blocks a fully funded order and then suggests a
        /// transfer the exchange refuses — a dead end for the caller.
        fn available_margin(&self) -> f64 {
            if self.unified() { self.perp + self.spot } else { self.perp }
        }
    }

    let balances_opt: Option<Balances> = if let Some(ref w) = wallet_opt {
        let aw_clone = arb_wallet_opt.clone();
        // HIP-3: when ordering on a builder DEX, perp margin must come from THAT dex's
        // clearinghouse — funds are NOT shared with the default DEX. If user has a
        // builder DEX coin (xyz:CL), query xyz's clearinghouse for the perp balance.
        let dex_clone = dex_opt.clone();
        let (perp_res, spot_res, arb_res, mode_res) = tokio::join!(
            get_clearinghouse_state_for_dex(info, w, dex_clone.as_deref()),
            get_spot_clearinghouse_state(info, w),
            async move {
                match aw_clone.as_deref() {
                    Some(aw) => erc20_balance(USDC_ARBITRUM, aw, arbitrum_rpc()).await,
                    None => Ok(0u128),
                }
            },
            crate::api::info_post(info, serde_json::json!({"type": "userAbstraction", "user": w})),
        );

        let arb_raw = match arb_res {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::execution_error_response(
                    &format!("Failed to query Arbitrum USDC balance: {:#}", e),
                    "RPC_ERROR",
                    "Check your Arbitrum connection and retry.",
                    is_autotrade,
                ));
                return Ok(());
            }
        };

        let perp = perp_res
            .ok()
            .and_then(|s| s["withdrawable"].as_str()?.parse::<f64>().ok())
            .unwrap_or(0.0);

        let spot = spot_res
            .ok()
            .and_then(|s| {
                s["balances"].as_array()?.iter()
                    .find(|b| b["coin"].as_str() == Some("USDC"))?
                    ["total"]
                    .as_str()?
                    .parse::<f64>()
                    .ok()
            })
            .unwrap_or(0.0);

        let mode = mode_res
            .ok()
            .map(|v| v.as_str().map(String::from).unwrap_or_else(|| v.to_string()))
            .unwrap_or_default();

        Some(Balances { perp, spot, arb: arb_raw as f64 / 1_000_000.0, mode })
    } else {
        None
    };

    // Estimate required margin; default to 10x if --leverage not provided.
    // Reduce-only orders close existing positions (release margin, don't add) — the
    // balance gate below would otherwise wrongly reject reduce-only attempts when the
    // account is near liquidation (perp balance low because losing position is tying
    // up collateral). Treat required_margin as 0 for reduce-only.
    let effective_leverage = args.leverage.map(|l| l as f64).unwrap_or(10.0);
    let required_margin = if args.reduce_only {
        0.0
    } else if notional > 0.0 {
        notional / effective_leverage
    } else {
        0.0
    };

    // Build balance landscape JSON (included in preview + stop output)
    let balance_json = balances_opt.as_ref().map(|b| {
        serde_json::json!({
            "perp_withdrawable": format!("{:.4}", b.perp),
            "spot_usdc":         format!("{:.4}", b.spot),
            "arbitrum_usdc":     format!("{:.4}", b.arb),
            "total_usdc":        format!("{:.4}", b.perp + b.spot + b.arb),
            "margin_mode":       b.mode,
            "available_margin":  format!("{:.4}", b.available_margin()),
        })
    });

    // Gate: STOP if perp balance is clearly insufficient.
    //
    // Tip text differs sharply between default DEX and HIP-3 builder DEX. On a
    // builder DEX, `b.perp` is the BUILDER DEX's clearinghouse balance (each is
    // isolated by HIP-3 design); naively suggesting `deposit` would route funds
    // into default DEX where they cannot back the order. Builder DEX users need
    // either dex-transfer or `abstraction --set unified`.
    //
    // Also: HL bridge minimum is $5; deposit smaller amounts loses funds.
    // Cap any `deposit` suggestion to `>= $5`.
    const HL_BRIDGE_MIN_USD: f64 = 5.0;
    if let Some(ref b) = balances_opt {
        if b.available_margin() < required_margin {
            let shortfall = required_margin - b.available_margin();
            let (error_code, tip) = if let Some(ref dex) = dex_opt {
                let dex = dex.as_str();
                let head = format!(
                    "{} DEX has its own isolated clearinghouse (HIP-3) — funds on default DEX or other DEXs do NOT back orders here. Two options:",
                    dex
                );
                let opt_a = "(A) Enable cross-DEX margin abstraction (one-time, then all DEXs share margin pool — this is HL Web UI's default behavior): `hyperliquid-plugin abstraction --set unified --confirm`".to_string();
                let opt_b = if b.spot >= shortfall {
                    format!(
                        "(B) Move ${:.2} from spot via default DEX: `hyperliquid-plugin transfer --amount {:.2} --direction spot-to-perp --confirm` then `hyperliquid-plugin dex-transfer --to-dex {} --amount {:.2} --confirm`",
                        shortfall, shortfall, dex, shortfall
                    )
                } else if b.arb >= shortfall.max(HL_BRIDGE_MIN_USD) {
                    let bridge_amt = shortfall.max(HL_BRIDGE_MIN_USD);
                    format!(
                        "(B) Bridge ${:.2} from Arbitrum (HL bridge minimum is $5): `hyperliquid-plugin deposit --amount {:.2} --confirm` then `hyperliquid-plugin dex-transfer --to-dex {} --amount {:.2} --confirm`",
                        bridge_amt, bridge_amt, dex, shortfall
                    )
                } else {
                    format!(
                        "(B) Liquid USDC across all wallets is ${:.2} — top up Arbitrum to ≥$5 first, then deposit + dex-transfer to {}. Spot USDH ${:.2} can be sold via `spot-order --coin USDH --side sell --type market --size <USDH amt>` (mind $10 min notional).",
                        b.spot + b.arb,
                        dex,
                        0.0  // we don't have spot USDH in Balances struct; placeholder
                    )
                };
                (
                    "BUILDER_DEX_UNFUNDED",
                    format!("{}\n{}\n{}", head, opt_a, opt_b),
                )
            } else if !b.unified() && b.spot >= shortfall {
                (
                    "PERP_INSUFFICIENT_BALANCE",
                    format!(
                        "Spot has enough USDC. Run: `hyperliquid-plugin transfer --amount {:.2} --direction spot-to-perp --confirm`",
                        shortfall
                    ),
                )
            } else if b.arb >= shortfall.max(HL_BRIDGE_MIN_USD) {
                let bridge_amt = shortfall.max(HL_BRIDGE_MIN_USD);
                (
                    "PERP_INSUFFICIENT_BALANCE",
                    format!(
                        "Arbitrum has enough USDC. Run: `hyperliquid-plugin deposit --amount {:.2} --confirm` (HL bridge minimum is $5; smaller deposits lose funds).",
                        bridge_amt
                    ),
                )
            } else {
                (
                    "PERP_INSUFFICIENT_BALANCE",
                    format!(
                        "Total liquid USDC: ${:.2}. Need ${:.2} more. HL bridge minimum is $5 — smaller deposits lose funds.",
                        b.perp + b.spot + b.arb,
                        shortfall
                    ),
                )
            };
            let tip = super::execution_failure_suggestion(is_autotrade, &tip);
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": "Insufficient perp balance",
                    "error_code": error_code,
                    "notional_usd": format!("${:.2}", notional),
                    "estimated_leverage": format!("{}x", effective_leverage as u32),
                    "required_margin_est": format!("${:.4}", required_margin),
                    "shortfall": format!("${:.4}", shortfall),
                    "fund_landscape": balance_json,
                    "tip": tip,
                }))?
            );
            return Ok(());
        }
    }

    // ─── Build action ────────────────────────────────────────────────────────
    let has_bracket = args.sl_px.is_some() || args.tp_px.is_some();

    let action = if has_bracket {
        let entry_element = match args.r#type.as_str() {
            "market" => serde_json::json!({
                "a": asset_idx,
                "b": is_buy,
                "p": slippage_px_str,
                "s": size_str,
                "r": args.reduce_only,
                "t": { "limit": { "tif": "Ioc" } }
            }),
            "limit" => {
                let price_str = match args.price.as_deref() {
                    Some(p) => p,
                    None => {
                        println!("{}", super::execution_error_response("--price is required for limit orders", "INVALID_ARGUMENT", "Provide a limit price, e.g. --price 100000", is_autotrade));
                        return Ok(());
                    }
                };
                if price_str.parse::<f64>().is_err() {
                    println!("{}", super::execution_error_response(&format!("Invalid price '{}'", price_str), "INVALID_ARGUMENT", "Provide a numeric price value.", is_autotrade));
                    return Ok(());
                }
                serde_json::json!({
                    "a": asset_idx,
                    "b": is_buy,
                    "p": price_str,
                    "s": size_str,
                    "r": args.reduce_only,
                    "t": { "limit": { "tif": "Gtc" } }
                })
            }
            _ => {
                println!("{}", super::execution_error_response(&format!("Unknown order type '{}'", args.r#type), "INVALID_ARGUMENT", "Use --type market or --type limit.", is_autotrade));
                return Ok(());
            }
        };

        build_bracketed_order_action(
            entry_element,
            asset_idx,
            is_buy,
            &size_str,
            sl_px_str.as_deref(),
            tp_px_str.as_deref(),
            sz_decimals,
            args.trigger_slippage,
        )
    } else {
        match args.r#type.as_str() {
            "market" => build_market_order_action(asset_idx, is_buy, &size_str, args.reduce_only, &slippage_px_str),
            "limit" => {
                let price_str = match args.price.as_deref() {
                    Some(p) => p,
                    None => {
                        println!("{}", super::execution_error_response("--price is required for limit orders", "INVALID_ARGUMENT", "Provide a limit price, e.g. --price 100000", is_autotrade));
                        return Ok(());
                    }
                };
                if price_str.parse::<f64>().is_err() {
                    println!("{}", super::execution_error_response(&format!("Invalid price '{}'", price_str), "INVALID_ARGUMENT", "Provide a numeric price value.", is_autotrade));
                    return Ok(());
                }
                build_limit_order_action(asset_idx, is_buy, price_str, &size_str, args.reduce_only, "Gtc")
            }
            _ => {
                println!("{}", super::execution_error_response(&format!("Unknown order type '{}'", args.r#type), "INVALID_ARGUMENT", "Use --type market or --type limit.", is_autotrade));
                return Ok(());
            }
        }
    };

    let leverage_preview = args.leverage.map(|l| {
        format!("{}x {}{}", l,
            if use_isolated { "isolated" } else { "cross" },
            if auto_isolated { " (auto, onlyIsolated)" } else { "" })
    });

    // ─── Preview ─────────────────────────────────────────────────────────────
    let mut preview_obj = serde_json::json!({
        "coin": coin,
        "assetIndex": asset_idx,
        "side": args.side,
        "size": size_str,
        "notional_usd": format!("{:.2}", notional),
        "type": args.r#type,
        "price": args.price,
        "leverage": leverage_preview,
        "slippagePct": args.slippage,
        "worstFillPrice": if args.r#type == "market" { Some(slippage_px_str.clone()) } else { None },
        "stopLoss": sl_px_str,
        "takeProfit": tp_px_str,
        "reduceOnly": args.reduce_only,
        "currentMidPrice": current_price,
        "grouping": if has_bracket { "normalTpsl" } else { "na" },
        "nonce": nonce
    });
    if let Some(ref bj) = balance_json {
        preview_obj["fund_landscape"] = bj.clone();
    }
    if let Some(job_id) = args.autotrade_job.as_deref() {
        preview_obj["autotradeJob"] = serde_json::Value::String(job_id.to_string());
        if args.dry_run {
            // Say it out loud, otherwise a dry-run preview reads as if it had been authorized.
            preview_obj["autotradeGrantCheck"] =
                serde_json::Value::String("skipped (dry-run)".to_string());
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "preview": preview_obj,
            "action": action
        }))?
    );

    if args.dry_run {
        eprintln!("\n[DRY RUN] Order not signed or submitted.");
        return Ok(());
    }

    if !args.confirm {
        eprintln!("\n[PREVIEW] Add --confirm to sign and submit this order.");
        eprintln!("WARNING: This will place a real perpetual order on Hyperliquid.");
        eprintln!("         Perpetuals trading involves significant risk including total loss.");
        return Ok(());
    }

    // Authorization must clear before anything is signed — including the leverage
    // update below, which is its own signed action.
    if let Some(job_id) = args.autotrade_job.as_deref() {
        let limit_px = args.price.as_deref().unwrap_or("");
        let prices = [current_price, slippage_px_str.as_str(), limit_px];
        let amount = match super::autotrade_quote_amount(&size_str, &prices) {
            Some(a) => a,
            None => {
                println!("{}", super::autotrade_amount_unavailable_response());
                return Ok(());
            }
        };
        if let Err(rejection) =
            super::autotrade_gate(job_id, &args.side, &amount, args.dry_run).await
        {
            println!("{}", rejection);
            return Ok(());
        }
    }

    // ─── Submit ───────────────────────────────────────────────────────────────
    let wallet = match wallet_opt {
        Some(w) => w,
        None => {
            println!("{}", super::execution_error_response("Cannot resolve wallet. Log in via onchainos.", "WALLET_NOT_FOUND", "Run onchainos wallet addresses to verify login.", is_autotrade));
            return Ok(());
        }
    };

    // Set leverage before placing the order if --leverage was provided
    if let Some(lev) = args.leverage {
        // HIP-3: respect onlyIsolated auto-promotion (xyz:CL / xyz:HOOD / etc.)
        let is_cross = !use_isolated;
        let lev_action = build_update_leverage_action(asset_idx, is_cross, lev);
        let lev_nonce = now_ms();
        let lev_signed = match onchainos_hl_sign(&lev_action, lev_nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::execution_error_response(&format!("Leverage update signing failed: {:#}", e), "SIGNING_FAILED", "Retry the command.", is_autotrade));
                return Ok(());
            }
        };
        let lev_result = match submit_exchange_request(exchange, lev_signed).await {
            Ok(v) => v,
            Err(e) => {
                println!("{}", super::execution_error_response(&format!("Leverage update failed: {:#}", e), "TX_SUBMIT_FAILED", "Retry the command.", is_autotrade));
                return Ok(());
            }
        };
        if lev_result["status"].as_str() == Some("err") {
            println!("{}", super::execution_error_response(
                &format!("Leverage update rejected: {}", lev_result["response"].as_str().unwrap_or("unknown error")),
                "TX_SUBMIT_FAILED",
                "Check your leverage settings and retry.",
                is_autotrade,
            ));
            return Ok(());
        }
        eprintln!(
            "Leverage set to {}x ({}) for {}",
            lev, if is_cross { "cross" } else { "isolated" }, coin
        );
    }

    let signed = match onchainos_hl_sign(&action, nonce, &wallet, ARBITRUM_CHAIN_ID, true, false) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::execution_error_response(&format!("{:#}", e), "SIGNING_FAILED", "Retry the command. If the issue persists, check onchainos status.", is_autotrade));
            return Ok(());
        }
    };
    let result = match submit_exchange_request(exchange, signed).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::execution_error_response(&format!("{:#}", e), "TX_SUBMIT_FAILED", "Retry the command. If the issue persists, check onchainos status.", is_autotrade));
            return Ok(());
        }
    };

    // Extract fill data for programmatic consumers (e.g. liquidation scripts)
    let statuses = result["response"]["data"]["statuses"]
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let avg_px = statuses["filled"]["avgPx"].as_str().map(|s| s.to_string());
    let oid = statuses["filled"]["oid"]
        .as_u64()
        .or_else(|| statuses["resting"]["oid"].as_u64());

    // Attribution: report every order that produced an oid (filled or resting).
    // strategy_id is optional — when not provided, an empty string is sent so the backend
    // still receives a record (just unattributed to any specific strategy).
    if let Some(oid_val) = oid {
        let ts_now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("system clock error for attribution: {}", e))?
            .as_secs();
        let sid = args.strategy_id.as_deref().unwrap_or("");
        let report_payload = serde_json::json!({
            "wallet": wallet,
            "proxyAddress": "",
            "order_id": oid_val.to_string(),
            "tx_hashes": [],
            "market_id": coin,
            "asset_id": "",
            "side": if is_buy { "BUY" } else { "SELL" },
            "amount": size_str,
            "symbol": "USDC",
            "price": avg_px.clone().unwrap_or_else(|| args.price.clone().unwrap_or_default()),
            "timestamp": ts_now,
            "strategy_id": sid,
            "plugin_name": "hyperliquid-plugin",
        });
        if let Err(e) = report_plugin_info(&report_payload) {
            eprintln!("[hyperliquid] Warning: report-plugin-info failed: {}", e);
        }
    }

    let mut success = serde_json::json!({
        "ok": true,
        "coin": coin,
        "side": args.side,
        "size": size_str,
        "notional_usd": format!("{:.2}", notional),
        "type": args.r#type,
        "stopLoss": sl_px_str,
        "takeProfit": tp_px_str,
        "data": {
            "avg_px": avg_px,
            "fill_px": avg_px,
            "oid": oid,
        },
        "result": result
    });
    if let Some(job_id) = args.autotrade_job.as_deref() {
        success["autotradeJob"] = serde_json::Value::String(job_id.to_string());
    }
    println!("{}", serde_json::to_string_pretty(&success)?);

    Ok(())
}

// ── order sizing helpers ────────────────────────────────────────────────────

/// Hyperliquid rejects any order worth less than this with
/// `Order must have minimum value of 10 USDC`.
pub const MIN_ORDER_USD: f64 = 10.0;

/// The price the exchange minimum is charged against.
///
/// A limit order's value is `size * limit price` — verified against the live
/// exchange: a HYPE sell limit at 99 with mid 81.8 and size 0.11 (0.11*99 =
/// $10.89, 0.11*mid = $9.00) was accepted, so the mid is not what HL measures.
/// Market orders carry no price of their own, so they use the mid.
pub fn notional_price(order_type: &str, limit_price: Option<&str>, mid: f64) -> f64 {
    if !order_type.eq_ignore_ascii_case("limit") {
        return mid;
    }
    limit_price
        .and_then(|p| p.trim().parse::<f64>().ok())
        .filter(|p| *p > 0.0)
        .unwrap_or(mid)
}

/// Smallest size on the coin's `szDecimals` grid whose value at `price` reaches
/// `target_usd`.
pub fn min_grid_size(target_usd: f64, price: f64, sz_decimals: u32) -> f64 {
    if price <= 0.0 {
        return 0.0;
    }
    let factor = 10_f64.powi(sz_decimals as i32);
    (target_usd / price * factor).ceil() / factor
}

#[cfg(test)]
mod sizing_tests {
    use super::*;

    #[test]
    fn limit_orders_are_priced_at_their_own_limit() {
        // The bug: a sell limit 21% above mid was sized by the mid, so the
        // order's real value overshot the $10 minimum it was sized for.
        assert_eq!(notional_price("limit", Some("99"), 82.0), 99.0);
        assert_eq!(notional_price("LIMIT", Some("99"), 82.0), 99.0);
    }

    #[test]
    fn market_orders_are_priced_at_the_mid() {
        assert_eq!(notional_price("market", None, 82.0), 82.0);
        // A stray --price on a market order must not change its valuation.
        assert_eq!(notional_price("market", Some("99"), 82.0), 82.0);
    }

    #[test]
    fn unusable_limit_price_falls_back_to_the_mid() {
        assert_eq!(notional_price("limit", None, 82.0), 82.0);
        assert_eq!(notional_price("limit", Some(""), 82.0), 82.0);
        assert_eq!(notional_price("limit", Some("abc"), 82.0), 82.0);
        assert_eq!(notional_price("limit", Some("0"), 82.0), 82.0);
        assert_eq!(notional_price("limit", Some("-5"), 82.0), 82.0);
    }

    #[test]
    fn min_grid_size_clears_the_minimum_on_the_first_try() {
        // HYPE szDecimals=2 at the limit price: 0.11*99 = $10.89 >= $10,
        // whereas the old mid-based path produced 0.13 ($12.87 of real value).
        let sz = min_grid_size(MIN_ORDER_USD, 99.0, 2);
        assert!((sz - 0.11).abs() < 1e-9, "got {}", sz);
        assert!(sz * 99.0 >= MIN_ORDER_USD);

        // Same coin priced at the mid needs a bigger size.
        let sz_mid = min_grid_size(MIN_ORDER_USD, 82.1, 2);
        assert!((sz_mid - 0.13).abs() < 1e-9, "got {}", sz_mid);

        // BTC szDecimals=5.
        let sz_btc = min_grid_size(MIN_ORDER_USD, 77813.0, 5);
        assert!((sz_btc - 0.00013).abs() < 1e-9, "got {}", sz_btc);
        assert!(sz_btc * 77813.0 >= MIN_ORDER_USD);
    }

    #[test]
    fn min_grid_size_is_safe_on_a_missing_price() {
        assert_eq!(min_grid_size(MIN_ORDER_USD, 0.0, 2), 0.0);
        assert_eq!(min_grid_size(MIN_ORDER_USD, -1.0, 2), 0.0);
    }
}
