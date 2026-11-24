use clap::Args;
use serde_json::{json, Value};

use crate::config::{ARB_KNOWN_MARKETS, ChainInfo, SUPPORTED_CHAINS, token_decimals};
use crate::onchainos::resolve_wallet;
use crate::rpc::{erc20_balance, fmt_token_amount, get_account_wei, get_earnings_rate, get_market_borrow_rate, native_balance, rate_to_apy, supply_rate_from};

const ABOUT: &str = "Dolomite is a decentralized money market and margin protocol on Arbitrum (also live on Berachain / Polygon zkEVM / X Layer / Mantle, but onchainos signing is currently Arbitrum-only). Supply assets to earn interest, open isolated borrow positions with up to 32 collateral assets each, and repay/withdraw via the DolomiteMargin core.";

/// Minimum native ETH (in wei) to be considered "fundable" for any write op on Arbitrum.
/// 0.0005 ETH (~$1.15) covers approve + main tx with comfortable headroom.
const ARB_GAS_FLOOR_WEI: u128 = 500_000_000_000_000;

/// Per-token dust threshold (atomic units) for "has supply" detection.
/// Anything below this is considered a leftover / not actively earning.
const STABLE_DUST_USD: u128 = 1_000_000; // $1 in 6-dec stablecoin atomic
const ETH_DUST_WEI: u128 = 1_000_000_000_000_000; // 0.001 ETH

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

/// Brief per-market snapshot (used to drive status decision + display).
struct MarketSnapshot {
    market_id: u128,
    symbol: &'static str,
    decimals: u32,
    /// Wallet's wallet-balance for this token (NOT yet supplied to Dolomite).
    wallet_balance_raw: u128,
    /// Account 0's supply in this market (Dolomite-internal). 0 if no position.
    supply_raw: u128,
    /// Account 0's borrow in this market (Dolomite-internal). 0 if no position.
    borrow_raw: u128,
    /// Live supply APY (decimal, e.g. 0.06 = 6%).
    supply_apy: Option<f64>,
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0]; // Arbitrum (only chain in v0.1.0)

    // 1. Resolve wallet
    let wallet = match &args.address {
        Some(a) => a.clone(),
        None => match resolve_wallet(chain.id) {
            Ok(a) => a,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("Could not resolve wallet from onchainos: {:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run `onchainos wallet addresses` to verify login, or pass --address explicitly.",
                ));
                return Ok(());
            }
        },
    };

    eprintln!("Scanning Dolomite state on Arbitrum for {}...", &wallet[..std::cmp::min(10, wallet.len())]);

    // 2. Read earnings_rate ONCE (global, supply rate derivation needs it),
    //    plus parallel native gas + per-token wallet/supply/borrow scan
    let native_fut = native_balance(&wallet, chain.rpc);
    let earnings_fut = get_earnings_rate(chain.dolomite_margin, chain.rpc);
    let (native_res, earnings_res) = tokio::join!(native_fut, earnings_fut);
    let earnings_rate = earnings_res.unwrap_or(850_000_000_000_000_000); // 85% fallback

    let market_futs: Vec<_> = ARB_KNOWN_MARKETS.iter().map(|(mid, sym, addr)| {
        let chain = chain.clone();
        let wallet = wallet.clone();
        async move {
            scan_market(*mid, sym, addr, &chain, &wallet, earnings_rate).await
        }
    }).collect();

    let market_results = futures::future::join_all(market_futs).await;

    // EVM-012: native gas balance failure must surface as RPC error rather
    // than misroute to `insufficient_gas` on every public-RPC blip.
    let native_bal = match native_res {
        Ok(v) => v,
        Err(e) => {
            println!("{}", super::error_response(
                &format!("Failed to read native balance on {}: {:#}", chain.key, e),
                "RPC_ERROR",
                "Public RPC may be limited; retry shortly.",
            ));
            return Ok(());
        }
    };

    // 3. Aggregate
    let mut rpc_failures = 0;
    let snapshots: Vec<MarketSnapshot> = market_results.into_iter().filter_map(|r| {
        match r {
            Ok(s) => Some(s),
            Err(_) => { rpc_failures += 1; None }
        }
    }).collect();

    let any_supply: bool = snapshots.iter().any(|s| has_dust_above(&s.symbol, s.supply_raw, s.decimals));
    let any_borrow: bool = snapshots.iter().any(|s| s.borrow_raw > 0);
    let any_wallet_balance: bool = snapshots.iter().any(|s| has_dust_above(&s.symbol, s.wallet_balance_raw, s.decimals));

    // 4. Status decision
    let (status, next_command, tip): (&str, Option<String>, String) = if rpc_failures >= 3 {
        ("rpc_degraded", None,
         format!("{} of {} market reads failed. Public Arbitrum RPC may be limited; retry in a minute.", rpc_failures, ARB_KNOWN_MARKETS.len()))
    } else if native_bal < ARB_GAS_FLOOR_WEI && !any_supply && !any_borrow {
        ("no_funds",
         Some("dolomite-plugin markets".to_string()),
         "Wallet has no Dolomite supply, no borrow, and no ETH gas. Top up at least 0.001 ETH on Arbitrum to start.".to_string())
    } else if any_borrow {
        // Most urgent: existing debt — show position so user can see health
        let b = snapshots.iter().find(|s| s.borrow_raw > 0).unwrap();
        ("has_borrow_position",
         Some(format!("dolomite-plugin positions --address {}", wallet)),
         format!("You have an active borrow position (e.g. {} {} borrowed). Run `positions` for the full health summary; if you want to close, use `repay`.",
            fmt_token_amount(b.borrow_raw, b.decimals), b.symbol)
        )
    } else if any_supply {
        // Earning yield, no borrow yet
        let s = best_supply(&snapshots).unwrap();
        let apy_str = s.supply_apy.map(|a| format!("{:.2}", a * 100.0)).unwrap_or_else(|| "?".to_string());
        ("has_supply_earning",
         Some(format!("dolomite-plugin positions --address {}", wallet)),
         format!("You're supplying {} {} earning ~{}% APY. Use `withdraw` to exit, or `borrow` to open a position against this collateral.",
            fmt_token_amount(s.supply_raw, s.decimals), s.symbol, apy_str)
        )
    } else if any_wallet_balance {
        // Has tokens in wallet but no Dolomite position
        let s = snapshots.iter().filter(|s| has_dust_above(&s.symbol, s.wallet_balance_raw, s.decimals))
            .max_by_key(|s| s.wallet_balance_raw).unwrap();
        let apy_str = s.supply_apy.map(|a| format!("{:.2}", a * 100.0)).unwrap_or_else(|| "?".to_string());
        let suggested_amt = sensible_supply_amount(s.wallet_balance_raw, s.decimals);
        ("ready_to_supply",
         Some(format!("dolomite-plugin supply --token {} --amount {} --confirm", s.symbol, suggested_amt)),
         format!("You have {} {} in wallet (Arbitrum). Supply to Dolomite to earn ~{}% APY.",
            fmt_token_amount(s.wallet_balance_raw, s.decimals), s.symbol, apy_str)
        )
    } else {
        // Has gas but no supportable token; suggest user explore markets
        ("needs_token",
         Some(format!("dolomite-plugin markets")),
         "You have ETH gas but no supportable tokens (USDC / USDT / WETH / DAI / WBTC / ARB). See `markets` for the full list, then top up one of them.".to_string())
    };

    // 5. Render
    let market_summaries: Vec<Value> = snapshots.iter().map(|s| {
        json!({
            "market_id": s.market_id,
            "symbol": s.symbol,
            "supply_apy_pct": s.supply_apy.map(|a| format!("{:.4}", a * 100.0)),
            "wallet_balance":      fmt_token_amount(s.wallet_balance_raw, s.decimals),
            "wallet_balance_raw":  s.wallet_balance_raw.to_string(),
            "supply_balance":      fmt_token_amount(s.supply_raw, s.decimals),
            "supply_balance_raw":  s.supply_raw.to_string(),
            "borrow_balance":      fmt_token_amount(s.borrow_raw, s.decimals),
            "borrow_balance_raw":  s.borrow_raw.to_string(),
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "about": ABOUT,
        "chain": chain.key,
        "chain_id": chain.id,
        "wallet": wallet,
        "rpc_failures": rpc_failures,
        "native_eth_balance": fmt_token_amount(native_bal, 18),
        "native_eth_balance_raw": native_bal.to_string(),
        "status": status,
        "next_command": next_command,
        "tip": tip,
        "markets_scanned": market_summaries.len(),
        "markets": market_summaries,
        "note": "Dolomite supports 1000+ assets. quickstart only scans the 7 most common markets (USDC/USDT/WETH/DAI/WBTC/ARB/USDC.e); for the full list use `markets`.",
    }))?);
    Ok(())
}

async fn scan_market(
    market_id: u64,
    symbol: &'static str,
    token_addr: &'static str,
    chain: &ChainInfo,
    wallet: &str,
    earnings_rate: u128,
) -> anyhow::Result<MarketSnapshot> {
    let decimals = token_decimals(symbol).unwrap_or(18);
    let mid_u128 = market_id as u128;

    // Three reads in parallel
    let bal_fut = erc20_balance(token_addr, wallet, chain.rpc);
    let pos_fut = get_account_wei(chain.dolomite_margin, wallet, 0, mid_u128, chain.rpc);
    let borrow_rate_fut = get_market_borrow_rate(chain.dolomite_margin, mid_u128, chain.rpc);

    let (bal_res, pos_res, borrow_res) = tokio::join!(bal_fut, pos_fut, borrow_rate_fut);

    // EVM-012: balance + position reads MUST propagate via `?` so the caller's
    // filter_map (which counts these as `rpc_failures` and may route to
    // `rpc_degraded`) sees them. Silent unwrap_or(0) used to make the status
    // decision tree fire on bad data.
    let wallet_bal = bal_res?;
    let (supply_raw, borrow_raw) = match pos_res? {
        (sign, value) if sign  => (value, 0),     // positive = supply
        (_,    value)          => (0, value),     // negative = borrow
    };
    // Supply APY = borrow_rate × earnings_rate / 1e18
    let supply_apy = borrow_res.ok()
        .map(|br| supply_rate_from(br, earnings_rate))
        .map(rate_to_apy);

    Ok(MarketSnapshot {
        market_id: mid_u128,
        symbol,
        decimals,
        wallet_balance_raw: wallet_bal,
        supply_raw,
        borrow_raw,
        supply_apy,
    })
}

/// Dust filter: > $1 USD-equivalent (rough — uses decimals as proxy for stables).
fn has_dust_above(symbol: &str, raw: u128, decimals: u32) -> bool {
    let upper = symbol.to_uppercase();
    if upper == "USDC" || upper == "USDC.E" || upper == "USDT" || upper == "DAI" {
        raw >= STABLE_DUST_USD * 10u128.pow(decimals.saturating_sub(6))
    } else if upper == "WETH" {
        raw >= ETH_DUST_WEI
    } else if upper == "WBTC" {
        raw >= 100_000  // 0.001 BTC
    } else if upper == "ARB" {
        raw >= 1_000_000_000_000_000_000  // 1 ARB
    } else {
        raw > 0
    }
}

fn best_supply(s: &[MarketSnapshot]) -> Option<&MarketSnapshot> {
    s.iter().filter(|m| m.supply_raw > 0).max_by_key(|m| m.supply_raw)
}

fn sensible_supply_amount(raw: u128, decimals: u32) -> String {
    // Round-down to a clean number, capped at 50 of any token for first-test feel.
    let factor = 10u128.pow(decimals);
    let whole = raw / factor;
    let cap = 50;
    let pick = whole.min(cap).max(1);
    pick.to_string()
}
