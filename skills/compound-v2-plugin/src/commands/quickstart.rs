use clap::Args;
use serde_json::{json, Value};

use crate::config::{ChainInfo, CTokenInfo, ETH_KNOWN_MARKETS, SUPPORTED_CHAINS};
use crate::onchainos::resolve_wallet;
use crate::rpc::{
    balance_of_underlying, borrow_balance_current, borrow_rate_per_block, fmt_token_amount,
    get_comp_accrued, is_borrow_paused, is_mint_paused, native_balance, rate_per_block_to_apr,
    supply_rate_per_block,
};

const ABOUT: &str = "Compound V2 — original cToken-based money market on Ethereum mainnet. As of 2026 governance has paused new supply on all 6 major markets (cDAI/cUSDC/cUSDT/cETH/cWBTC2/cCOMP). This plugin is positioned as an EXIT TOOL: redeem existing cToken positions, repay legacy debt, claim accrued COMP. For new supply/borrow on Compound, install compound-v3-plugin (V3/Comet — actively maintained).";

/// Minimum native ETH (wei) to be considered "fundable" for any write op on Ethereum mainnet.
/// Compound V2 calls are L1, gas is expensive — 0.005 ETH (~$15-20) covers approve + main tx.
const ETH_GAS_FLOOR_WEI: u128 = 5_000_000_000_000_000;

/// Supply position threshold (atomic units) for "has supply" detection (per-decimal class).
/// Below this we treat as dust / not actively earning.
const STABLE_DUST_USD: u128 = 1_000_000;             // $1 in 6-dec stable
const ETH_DUST_WEI: u128 = 1_000_000_000_000_000;     // 0.001 ETH
const WBTC_DUST: u128 = 100_000;                       // 0.001 BTC (8 dec)
const ERC18_DUST: u128 = 1_000_000_000_000_000_000;    // 1 unit of 18-dec token (DAI / COMP)

/// COMP accrued threshold to surface as `has_comp_accrued` (5e16 ≈ 0.05 COMP).
/// Below this, claim gas (~$5-10 L1) outweighs the rewards.
const COMP_DUST_THRESHOLD: u128 = 50_000_000_000_000_000;

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

struct MarketSnapshot {
    info: &'static CTokenInfo,
    /// Wallet's underlying-token balance (NOT yet supplied).
    wallet_balance_raw: u128,
    /// User's current supply position in underlying units (0 if no supply).
    supply_underlying_raw: u128,
    /// User's current borrow position in underlying units (0 if no debt).
    borrow_underlying_raw: u128,
    supply_apr: Option<f64>,
    borrow_apr: Option<f64>,
    mint_paused: bool,
    borrow_paused: bool,
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    let chain = &SUPPORTED_CHAINS[0]; // Ethereum mainnet (only chain in v0.1.0)

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

    eprintln!("Scanning Compound V2 state on Ethereum for {}...", &wallet[..std::cmp::min(10, wallet.len())]);

    // 2. Native gas + accrued COMP (each independent) + per-market scan in parallel
    let native_fut = native_balance(&wallet, chain.rpc);
    let comp_fut = get_comp_accrued(chain.comptroller, &wallet, chain.rpc);

    let market_futs: Vec<_> = ETH_KNOWN_MARKETS.iter().map(|info| {
        let chain = chain.clone();
        let wallet = wallet.clone();
        async move { scan_market(info, &chain, &wallet).await }
    }).collect();

    let (native_res, comp_res, market_results) = tokio::join!(
        native_fut, comp_fut, futures::future::join_all(market_futs)
    );

    // EVM-012: critical reads (native gas) must surface as RPC errors. Silent
    // unwrap_or(0) here used to misroute users to `insufficient_gas` whenever
    // public RPC blipped, even when their wallet was actually well-funded.
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
    // COMP accrued is non-critical (only used for `has_rewards_accrued` status).
    // Keep the 0 fallback but expose the error so callers can distinguish
    // "no rewards accrued" from "Comptroller RPC failed".
    let (comp_accrued_raw, comp_query_error) = match comp_res {
        Ok(v) => (v, None),
        Err(e) => (0u128, Some(format!("{:#}", e))),
    };

    let mut rpc_failures = 0;
    let snapshots: Vec<MarketSnapshot> = market_results.into_iter().filter_map(|r| match r {
        Ok(s) => Some(s),
        Err(_) => { rpc_failures += 1; None }
    }).collect();

    let any_supply: bool = snapshots.iter().any(|s| has_supply_above_dust(s));
    let any_borrow: bool = snapshots.iter().any(|s| s.borrow_underlying_raw > 0);
    let has_comp = comp_accrued_raw >= COMP_DUST_THRESHOLD;
    let has_gas = native_bal >= ETH_GAS_FLOOR_WEI;

    // 3. Status decision tree
    let (status, next_command, tip): (&str, Option<String>, String) = if rpc_failures >= 3 {
        ("rpc_degraded", None,
         format!("{} of {} market reads failed. Public Ethereum RPC may be limited; retry shortly.", rpc_failures, ETH_KNOWN_MARKETS.len()))
    } else if any_borrow {
        // Most urgent: existing debt
        let b = snapshots.iter().find(|s| s.borrow_underlying_raw > 0).unwrap();
        ("has_debt_can_repay",
         Some(format!("compound-v2-plugin repay --token {} --all --confirm", b.info.underlying_symbol)),
         format!("You have an active borrow position (e.g. {} {} debt). Repay-all uses uint256.max sentinel — settles to exact zero, no dust.",
            fmt_token_amount(b.borrow_underlying_raw, b.info.underlying_decimals), b.info.underlying_symbol)
        )
    } else if has_comp {
        ("has_comp_accrued",
         Some("compound-v2-plugin claim-comp --confirm".to_string()),
         format!("You have ~{} COMP accrued. Claim before unsupplying (claim is per-COMP-distribution-state, redeem can zero supply position which freezes accrual).",
            fmt_token_amount(comp_accrued_raw, 18))
        )
    } else if any_supply {
        let s = snapshots.iter().filter(|m| has_supply_above_dust(m))
            .max_by_key(|m| m.supply_underlying_raw).unwrap();
        let apr_str = s.supply_apr.map(|a| format!("{:.2}", a * 100.0)).unwrap_or_else(|| "?".into());
        ("has_supply_can_redeem",
         Some(format!("compound-v2-plugin withdraw --token {} --amount all --confirm", s.info.underlying_symbol)),
         format!("You have {} {} supplied (earning {}% APR). V2 is in wind-down — withdraw to wallet, then redeposit to compound-v3-plugin if you want to keep earning.",
            fmt_token_amount(s.supply_underlying_raw, s.info.underlying_decimals), s.info.underlying_symbol, apr_str)
        )
    } else if !has_gas && !any_supply && !any_borrow {
        ("insufficient_gas",
         None,
         format!("Wallet has only {} ETH gas. Compound V2 ops are L1 mainnet — top up at least 0.005 ETH (~$15) to interact.", fmt_token_amount(native_bal, 18))
        )
    } else {
        // No V2 history, has gas — wind-down redirect
        ("protocol_winddown",
         Some("npx skills add okx/plugin-store --skill compound-v3-plugin".to_string()),
         "You have no Compound V2 positions. All V2 supply markets are governance-paused (wind-down mode). Use compound-v3-plugin for active supply/borrow on the same Compound team's V3 (Comet).".to_string())
    };

    // 4. Render
    let market_summaries: Vec<Value> = snapshots.iter().map(|s| {
        json!({
            "ctoken": s.info.ctoken,
            "ctoken_symbol": s.info.symbol,
            "underlying": s.info.underlying_symbol,
            "supply_apr_pct": s.supply_apr.map(|a| format!("{:.4}", a * 100.0)),
            "borrow_apr_pct": s.borrow_apr.map(|a| format!("{:.4}", a * 100.0)),
            "mint_paused": s.mint_paused,
            "borrow_paused": s.borrow_paused,
            "wallet_balance":      fmt_token_amount(s.wallet_balance_raw, s.info.underlying_decimals),
            "wallet_balance_raw":  s.wallet_balance_raw.to_string(),
            "supply_underlying":      fmt_token_amount(s.supply_underlying_raw, s.info.underlying_decimals),
            "supply_underlying_raw":  s.supply_underlying_raw.to_string(),
            "borrow_underlying":      fmt_token_amount(s.borrow_underlying_raw, s.info.underlying_decimals),
            "borrow_underlying_raw":  s.borrow_underlying_raw.to_string(),
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "about": ABOUT,
        "chain": chain.key,
        "chain_id": chain.id,
        "wallet": wallet,
        "winddown_warning": "All 6 major Compound V2 markets have governance-paused new supply (mintGuardianPaused=true). This tool is for EXIT flows. Install compound-v3-plugin for active flows.",
        "rpc_failures": rpc_failures,
        "native_eth_balance": fmt_token_amount(native_bal, 18),
        "native_eth_balance_raw": native_bal.to_string(),
        "comp_accrued":     fmt_token_amount(comp_accrued_raw, 18),
        "comp_accrued_raw": comp_accrued_raw.to_string(),
        "comp_query_error": comp_query_error,
        "status": status,
        "next_command": next_command,
        "tip": tip,
        "markets_scanned": market_summaries.len(),
        "markets": market_summaries,
        "note": "Compound V2 Comptroller at 0x3d9819210A31b4961b30EF54bE2aeD79B9c9Cd3B (Unitroller proxy). v0.1.0 covers 6 markets; full enumeration via `markets --all`.",
    }))?);
    Ok(())
}

async fn scan_market(info: &'static CTokenInfo, chain: &ChainInfo, wallet: &str) -> anyhow::Result<MarketSnapshot> {
    let bal_fut = async {
        if info.is_native {
            native_balance(wallet, chain.rpc).await
        } else {
            crate::rpc::erc20_balance(info.underlying, wallet, chain.rpc).await
        }
    };
    let supply_fut = balance_of_underlying(info.ctoken, wallet, chain.rpc);
    let borrow_fut = borrow_balance_current(info.ctoken, wallet, chain.rpc);
    let supply_rate_fut = supply_rate_per_block(info.ctoken, chain.rpc);
    let borrow_rate_fut = borrow_rate_per_block(info.ctoken, chain.rpc);
    let mint_paused_fut = is_mint_paused(chain.comptroller, info.ctoken, chain.rpc);
    let borrow_paused_fut = is_borrow_paused(chain.comptroller, info.ctoken, chain.rpc);

    let (bal, supply, borrow, sr, br, mp, bp) = tokio::join!(
        bal_fut, supply_fut, borrow_fut, supply_rate_fut, borrow_rate_fut, mint_paused_fut, borrow_paused_fut
    );

    // EVM-012: balance reads MUST propagate via `?` so the caller's filter_map
    // (which counts these as `rpc_failures` and may route to `rpc_degraded`)
    // sees them. Silent unwrap_or(0) used to make the status decision tree
    // fire on bad data — e.g. a single RPC blip on the user's debt token would
    // route them to `protocol_winddown` (no V2 positions) instead of
    // `has_active_borrow`.
    Ok(MarketSnapshot {
        info,
        wallet_balance_raw: bal?,
        supply_underlying_raw: supply?,
        borrow_underlying_raw: borrow?,
        supply_apr: sr.ok().map(|r| rate_per_block_to_apr(r, chain.blocks_per_year)),
        borrow_apr: br.ok().map(|r| rate_per_block_to_apr(r, chain.blocks_per_year)),
        mint_paused: mp.unwrap_or(false),
        borrow_paused: bp.unwrap_or(false),
    })
}

fn has_supply_above_dust(s: &MarketSnapshot) -> bool {
    let upper = s.info.underlying_symbol.to_uppercase();
    if upper == "USDC" || upper == "USDT" {
        s.supply_underlying_raw >= STABLE_DUST_USD
    } else if upper == "DAI" || upper == "COMP" {
        s.supply_underlying_raw >= ERC18_DUST
    } else if upper == "ETH" {
        s.supply_underlying_raw >= ETH_DUST_WEI
    } else if upper == "WBTC" {
        s.supply_underlying_raw >= WBTC_DUST
    } else {
        s.supply_underlying_raw > 0
    }
}
