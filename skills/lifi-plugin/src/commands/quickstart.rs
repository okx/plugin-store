use clap::Args;
use serde_json::{json, Value};

use crate::config::{ChainInfo, SUPPORTED_CHAINS};
use crate::onchainos::resolve_wallet;
use crate::rpc::{erc20_balance, fmt_token_amount, native_balance};

const ABOUT: &str = "LI.FI is a cross-chain bridge & swap aggregator. This skill lets you list chains/tokens, get quotes, plan multi-hop routes, execute bridges/swaps with a single signed tx, and track in-flight transfers across Ethereum, Arbitrum, Base, Optimism, BSC, and Polygon.";

/// Native USDC (or stablecoin-equivalent) per supported chain.
/// Returns (contract_address, decimals).
///
/// **NOTE on decimals**: BSC's "Binance-Peg USD Coin" uses 18 decimals, not 6
/// like every other chain's native USDC. Hard-coding decimals here avoids an
/// extra API roundtrip in the hot path; if a chain's USDC contract ever
/// changes decimals (extremely rare), update this table.
fn usdc_meta(chain_id: u64) -> Option<(&'static str, u32)> {
    match chain_id {
        1     => Some(("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6)),  // Ethereum native USDC
        42161 => Some(("0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6)),  // Arbitrum native USDC
        8453  => Some(("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", 6)),  // Base native USDC
        10    => Some(("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85", 6)),  // Optimism native USDC
        56    => Some(("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d", 18)), // BSC USDC (Binance-Peg, 18 dec!)
        137   => Some(("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359", 6)),  // Polygon native USDC
        _ => None,
    }
}

/// Per-chain balance snapshot used to drive the status decision.
struct ChainSnapshot {
    chain: &'static ChainInfo,
    native_raw: u128,
    usdc_raw: u128,
    usdc_decimals: u32,
    error: Option<String>,
}

impl ChainSnapshot {
    /// Approx USD value used only to pick the "richest chain" — assumes USDC ≈ $1
    /// and native gas tokens have no USD value (we don't have a price feed in this
    /// quickstart path, and the user only needs to know "where do I have stables").
    fn stable_usd(&self) -> f64 {
        if self.usdc_decimals == 0 || self.usdc_raw == 0 {
            return 0.0;
        }
        self.usdc_raw as f64 / 10f64.powi(self.usdc_decimals as i32)
    }

    fn has_any_balance(&self) -> bool {
        self.native_raw > 0 || self.usdc_raw > 0
    }
}

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    // ── 1. Resolve wallet ─────────────────────────────────────────────────────
    // EVM wallet is the same across all 6 chains (single key). We resolve via
    // the first chain (Ethereum) but accept any chain id since onchainos
    // returns the same address.
    let wallet = match &args.address {
        Some(addr) => addr.clone(),
        None => match resolve_wallet(1) {
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

    eprintln!("Scanning balances for {} on 6 chains...", &wallet[..std::cmp::min(10, wallet.len())]);

    // ── 2. Parallel native + USDC balance fetch across all 6 chains ──────────
    let snapshots = fetch_all_balances(&wallet).await;

    // ── 3. Decide status and next_command ─────────────────────────────────────
    let total_chains = snapshots.len();
    let rpc_failures = snapshots.iter().filter(|s| s.error.is_some()).count();
    let chains_with_balance: Vec<&ChainSnapshot> = snapshots.iter().filter(|s| s.has_any_balance()).collect();

    // Richest chain by USDC value (only chains that didn't error out).
    let richest = snapshots
        .iter()
        .filter(|s| s.error.is_none())
        .max_by(|a, b| a.stable_usd().partial_cmp(&b.stable_usd()).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|s| s.has_any_balance());

    let (status, next_command, tip) = if rpc_failures >= 4 {
        // 4+ of 6 RPCs failed — environment problem, not user-actionable
        (
            "rpc_degraded",
            None,
            "More than half the public RPCs failed to respond. Retry in a minute, or check connectivity.".to_string(),
        )
    } else if chains_with_balance.is_empty() {
        // Wallet exists but no funds anywhere
        (
            "no_funds",
            Some(format!(
                "lifi-plugin balance --address {}",
                wallet
            )),
            "Wallet has no native or USDC balance on any of the 6 supported chains. Top up native gas + USDC on at least one chain (Base or Arbitrum are typically cheapest).".to_string(),
        )
    } else if let Some(r) = richest {
        if let Some(amount_str) = sensible_test_amount(r.stable_usd()) {
            // Has enough stables for a test bridge.
            let target = pick_bridge_target(r.chain.id);
            let next = format!(
                "lifi-plugin bridge --from-chain {} --to-chain {} --from-token USDC --to-token USDC --amount {} --confirm",
                r.chain.key, target.key, amount_str
            );
            (
                "ready",
                Some(next),
                format!(
                    "You have {} USDC on {} ({}). Try a small {} USDC bridge to {} to test the flow end-to-end.",
                    fmt_token_amount(r.usdc_raw, r.usdc_decimals),
                    r.chain.key,
                    r.chain.name,
                    amount_str,
                    target.name,
                ),
            )
        } else {
            // Has some balance but no chain has enough USDC for a meaningful test bridge.
            (
                "low_balance",
                Some(format!("lifi-plugin balance --address {} --token USDC", wallet)),
                format!(
                    "Your richest chain ({}) has {} USDC — below the $5 minimum for a meaningful test bridge. Top up USDC, or use `lifi-plugin balance --token USDC` to inspect all chains.",
                    r.chain.key,
                    fmt_token_amount(r.usdc_raw, r.usdc_decimals),
                ),
            )
        }
    } else {
        // Defensive fallback: collapses to no_funds rather than introducing an
        // undocumented `unknown` status that SUMMARY.md would have to cover.
        // Prior branches already handle no-funds and degraded-RPC; reaching
        // here implies a logic gap, so we surface as no_funds (safest "do
        // nothing" recommendation for the user).
        (
            "no_funds",
            Some(format!("lifi-plugin balance --address {}", wallet)),
            "No actionable balance detected. Inspect per-chain balances to debug.".to_string(),
        )
    };

    // ── 4. Render structured output ───────────────────────────────────────────
    let chain_summaries: Vec<Value> = snapshots
        .iter()
        .map(|s| {
            if let Some(ref err) = s.error {
                json!({
                    "chain": s.chain.key,
                    "chain_id": s.chain.id,
                    "error": err,
                })
            } else {
                let mut entry = json!({
                    "chain": s.chain.key,
                    "chain_id": s.chain.id,
                    "native": {
                        "symbol": s.chain.native_symbol,
                        "amount": fmt_token_amount(s.native_raw, 18),
                        "amount_raw": s.native_raw.to_string(),
                    },
                });
                if s.usdc_decimals > 0 {
                    entry["usdc"] = json!({
                        "amount": fmt_token_amount(s.usdc_raw, s.usdc_decimals),
                        "amount_raw": s.usdc_raw.to_string(),
                        "decimals": s.usdc_decimals,
                        "usd_value": format!("{:.6}", s.stable_usd()),
                    });
                }
                entry
            }
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "scanned_chains": total_chains,
        "rpc_failures": rpc_failures,
        "richest_chain": richest.map(|r| r.chain.key),
        "status": status,
        "next_command": next_command,
        "tip": tip,
        "chains": chain_summaries,
    }))?);

    Ok(())
}

/// Fan out 6 chains × (native + optional USDC) = 12 RPC calls, all in parallel.
async fn fetch_all_balances(wallet: &str) -> Vec<ChainSnapshot> {
    let futures: Vec<_> = SUPPORTED_CHAINS
        .iter()
        .map(|chain| async move {
            let native_fut = native_balance(wallet, chain.rpc);
            let usdc_meta_local = usdc_meta(chain.id);
            let usdc_fut = usdc_meta_local.map(|(addr, _)| erc20_balance(addr, wallet, chain.rpc));

            // Run the two RPC calls for this chain concurrently.
            let (native_res, usdc_res) = match usdc_fut {
                Some(u) => {
                    let (n, u) = tokio::join!(native_fut, u);
                    (n, Some(u))
                }
                None => (native_fut.await, None),
            };

            let mut error = None;
            let native_raw = native_res.unwrap_or_else(|e| {
                error = Some(format!("native balance: {}", e));
                0
            });
            let usdc_raw = match usdc_res {
                Some(Ok(v)) => v,
                Some(Err(e)) => {
                    if error.is_none() {
                        error = Some(format!("USDC balance: {}", e));
                    }
                    0
                }
                None => 0,
            };

            let usdc_decimals = usdc_meta_local.map(|(_, d)| d).unwrap_or(0);
            ChainSnapshot { chain, native_raw, usdc_raw, usdc_decimals, error }
        })
        .collect();
    futures::future::join_all(futures).await
}

/// Pick a bridge destination different from the source chain.
/// Heuristic: if user is on a high-fee chain (ETH), suggest the cheapest L2 (Base).
/// Otherwise suggest Base as a generally cheap destination, falling back to Arbitrum.
fn pick_bridge_target(source_id: u64) -> &'static ChainInfo {
    let target_id = match source_id {
        1 => 8453,      // ETH → Base
        8453 => 42161,  // Base → Arbitrum
        _ => 8453,      // anything else → Base
    };
    SUPPORTED_CHAINS
        .iter()
        .find(|c| c.id == target_id)
        .unwrap_or(&SUPPORTED_CHAINS[2]) // safe fallback to Base (index 2)
}

/// Choose a human-readable test amount string given the user's USDC balance in dollars.
/// Returns None when the chain has < $5 USDC (caller falls through to low_balance).
fn sensible_test_amount(usdc_dollars: f64) -> Option<String> {
    if usdc_dollars >= 5.0 {
        Some("0.5".to_string())
    } else {
        None
    }
}
