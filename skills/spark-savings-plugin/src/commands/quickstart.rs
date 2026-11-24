use clap::Args;
use serde_json::{json, Value};

use crate::config::{ConvertMechanism, ChainInfo, SUPPORTED_CHAINS, STABLE_DECIMALS};
use crate::onchainos::resolve_wallet;
use crate::rpc::{erc20_balance, fmt_token_amount, native_balance, ssr_to_apy, susds_ssr};

const ABOUT: &str = "Spark Savings is the yield-bearing arm of Sky (formerly MakerDAO). Deposit USDS or DAI and receive sUSDS — an ERC-4626 vault token that auto-accrues the Sky Savings Rate (SSR). No collateral, no liquidation, just compounding stablecoin yield.";

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

/// Per-chain balance + mechanism snapshot.
struct ChainSnapshot {
    chain: &'static ChainInfo,
    native_raw: u128,
    usds_raw: u128,
    susds_raw: u128,
    dai_raw: u128, // only on Ethereum; 0 elsewhere
    error: Option<String>,
}

impl ChainSnapshot {
    /// Approx USD value: USDS / DAI / sUSDS-as-USDS-equivalent are all ~$1.
    /// We don't fetch a price feed; for picking "richest chain" this is fine.
    fn usd_value(&self) -> f64 {
        let factor = 10f64.powi(STABLE_DECIMALS as i32);
        let usds = self.usds_raw as f64 / factor;
        let susds = self.susds_raw as f64 / factor;  // ~1 USDS each
        let dai = self.dai_raw as f64 / factor;
        usds + susds + dai
    }

    fn has_actionable_balance(&self) -> bool {
        // USDS / DAI on a chain with a deposit mechanism; OR sUSDS anywhere (can be redeemed)
        let has_deposit_input = (self.usds_raw > 0 || self.dai_raw > 0)
            && self.chain.mechanism != ConvertMechanism::SparkPsm
                || (self.usds_raw > 0 && self.chain.spark_psm.is_some());
        let has_susds = self.susds_raw > 0;
        has_deposit_input || has_susds
    }
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    // ── 1. Resolve wallet ─────────────────────────────────────────────────────
    let wallet = match &args.address {
        Some(a) => a.clone(),
        None => match resolve_wallet(1) {
            Ok(a) => a,
            Err(e) => {
                println!("{}", super::error_response(
                    &format!("Could not resolve wallet from onchainos: {:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run `onchainos wallet addresses` to verify login, or pass --address.",
                ));
                return Ok(());
            }
        },
    };

    eprintln!("Scanning USDS / sUSDS / DAI on 3 chains for {}...", &wallet[..std::cmp::min(10, wallet.len())]);

    // ── 2. Parallel scan: 3 chains × (native + USDS + sUSDS + DAI on ETH only) ─
    let snapshots = fetch_all_balances(&wallet).await;

    // ── 3. Fetch live SSR/APY from Ethereum sUSDS ─────────────────────────────
    // SSR is set on mainnet; L2 sUSDS prices follow via oracle, so the rate is
    // effectively the same. Read from mainnet for the canonical APY.
    let eth_chain = SUPPORTED_CHAINS.iter().find(|c| c.id == 1).unwrap();
    let apy_pct = match susds_ssr(eth_chain.susds, eth_chain.rpc).await {
        Ok(ssr_ray) => Some(ssr_to_apy(ssr_ray) * 100.0),
        Err(_) => None,
    };

    // ── 4. Decide status + next_command ───────────────────────────────────────
    let total_chains = snapshots.len();
    let rpc_failures = snapshots.iter().filter(|s| s.error.is_some()).count();
    let any_actionable = snapshots.iter().any(|s| s.has_actionable_balance());

    // Richest chain by total stable-coin USD value (any kind: USDS / sUSDS / DAI)
    let richest = snapshots.iter()
        .filter(|s| s.error.is_none())
        .max_by(|a, b| a.usd_value().partial_cmp(&b.usd_value()).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|s| s.usd_value() > 0.0);

    let (status, next_command, tip): (&str, Option<String>, String) = if rpc_failures >= 2 {
        ("rpc_degraded", None,
         "More than half of public RPCs failed. Retry in a minute.".to_string())
    } else if !any_actionable {
        ("no_funds", Some(format!("spark-savings-plugin balance --address {}", wallet)),
         "No USDS / sUSDS / DAI on any of the 3 supported chains. Top up USDS on Ethereum, Base, or Arbitrum to start earning.".to_string())
    } else if let Some(r) = richest {
        decide_next(r, &wallet)
    } else {
        ("no_funds", Some(format!("spark-savings-plugin balance --address {}", wallet)),
         "Wallet has no Spark-relevant balances on the 3 supported chains.".to_string())
    };

    // ── 5. Render ─────────────────────────────────────────────────────────────
    let chain_summaries: Vec<Value> = snapshots.iter().map(|s| {
        if let Some(ref err) = s.error {
            return json!({
                "chain": s.chain.key,
                "chain_id": s.chain.id,
                "error": err,
            });
        }
        let mut entry = json!({
            "chain": s.chain.key,
            "chain_id": s.chain.id,
            "mechanism": match s.chain.mechanism {
                ConvertMechanism::Erc4626Vault => "ERC-4626 vault (deposit/redeem on sUSDS contract)",
                ConvertMechanism::SparkPsm     => "Spark PSM (swapExactIn USDS↔sUSDS)",
            },
            "native": {
                "symbol": s.chain.native_symbol,
                "amount": fmt_token_amount(s.native_raw, 18),
                "amount_raw": s.native_raw.to_string(),
            },
            "usds": {
                "amount": fmt_token_amount(s.usds_raw, STABLE_DECIMALS),
                "amount_raw": s.usds_raw.to_string(),
            },
            "susds": {
                "amount": fmt_token_amount(s.susds_raw, STABLE_DECIMALS),
                "amount_raw": s.susds_raw.to_string(),
            },
        });
        if s.chain.dai.is_some() {
            entry["dai"] = json!({
                "amount": fmt_token_amount(s.dai_raw, STABLE_DECIMALS),
                "amount_raw": s.dai_raw.to_string(),
            });
        }
        entry
    }).collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "about": ABOUT,
        "wallet": wallet,
        "scanned_chains": total_chains,
        "rpc_failures": rpc_failures,
        "current_apy_pct": apy_pct.map(|v| format!("{:.4}", v)),
        "richest_chain": richest.map(|r| r.chain.key),
        "status": status,
        "next_command": next_command,
        "tip": tip,
        "chains": chain_summaries,
    }))?);
    Ok(())
}

/// Decide next_command based on the richest chain's holdings.
/// Status enum (covered by SUMMARY.md):
///   - has_susds_redeemable: user already has sUSDS, can redeem to USDS
///   - has_dai_to_upgrade:   user has DAI on Ethereum, should upgrade to USDS first
///   - ready_to_deposit:     user has USDS, deposit to start earning
fn decide_next(r: &ChainSnapshot, wallet: &str) -> (&'static str, Option<String>, String) {
    let usds_human  = fmt_token_amount(r.usds_raw, STABLE_DECIMALS);
    let susds_human = fmt_token_amount(r.susds_raw, STABLE_DECIMALS);
    let dai_human   = fmt_token_amount(r.dai_raw, STABLE_DECIMALS);

    // Priority 1: existing sUSDS earning yield → show balance + redeem hint
    if r.susds_raw > 0 {
        return (
            "has_susds_earning",
            Some(format!("spark-savings-plugin balance --address {} --chain {}", wallet, r.chain.key)),
            format!(
                "You have {} sUSDS on {} ({}) actively earning SSR. To redeem: `spark-savings-plugin withdraw --chain {} --amount <X> --confirm`. To check accrued yield: `spark-savings-plugin balance --chain {}`.",
                susds_human, r.chain.key, r.chain.name, r.chain.key, r.chain.key
            ),
        );
    }

    // Priority 2: DAI on Ethereum → upgrade-dai first (only Ethereum has DaiUsds)
    if r.dai_raw > 0 && r.chain.dai_usds_migrator.is_some() {
        let suggested = if r.dai_raw > 10u128.pow(STABLE_DECIMALS) { "10" } else { "1" };
        return (
            "has_dai_to_upgrade",
            Some(format!(
                "spark-savings-plugin upgrade-dai --amount {} --confirm",
                suggested,
            )),
            format!(
                "You have {} DAI on Ethereum. Spark uses USDS (DAI's successor); upgrade 1:1 atomically via the official DaiUsds migrator, then deposit. Free, no slippage.",
                dai_human
            ),
        );
    }

    // Priority 3: USDS available → deposit to start earning
    if r.usds_raw > 0 {
        let suggested_amount = sensible_deposit_size(r.usds_raw);
        return (
            "ready_to_deposit",
            suggested_amount.as_ref().map(|amt| format!(
                "spark-savings-plugin deposit --chain {} --amount {} --confirm",
                r.chain.key, amt,
            )),
            format!(
                "You have {} USDS on {} ({}). Deposit to start earning Sky Savings Rate. Mechanism: {}.",
                usds_human, r.chain.key, r.chain.name,
                match r.chain.mechanism {
                    ConvertMechanism::Erc4626Vault => "direct ERC-4626 deposit on sUSDS",
                    ConvertMechanism::SparkPsm     => "Spark PSM swapExactIn",
                }
            ),
        );
    }

    // Defensive
    ("no_funds", None, "Unexpected: richest chain has no Spark-relevant balance.".to_string())
}

/// Suggest a sensible test deposit amount based on user's USDS balance.
fn sensible_deposit_size(usds_raw: u128) -> Option<String> {
    let factor = 10u128.pow(STABLE_DECIMALS);
    let dollars = usds_raw / factor;
    if dollars >= 100 { Some("10".to_string()) }
    else if dollars >= 10 { Some("1".to_string()) }
    else if dollars >= 1 { Some("0.5".to_string()) }
    else { None }
}

async fn fetch_all_balances(wallet: &str) -> Vec<ChainSnapshot> {
    let futs: Vec<_> = SUPPORTED_CHAINS.iter().map(|chain| async move {
        let native_fut = native_balance(wallet, chain.rpc);
        let usds_fut = erc20_balance(chain.usds, wallet, chain.rpc);
        let susds_fut = erc20_balance(chain.susds, wallet, chain.rpc);
        // DAI exists only on Ethereum
        let dai_fut = chain.dai.map(|addr| erc20_balance(addr, wallet, chain.rpc));

        let (n, u, s) = tokio::join!(native_fut, usds_fut, susds_fut);
        let dai_res = if let Some(f) = dai_fut { Some(f.await) } else { None };

        let mut error = None;
        let native_raw = n.unwrap_or_else(|e| {
            error = Some(format!("native: {}", e));
            0
        });
        let usds_raw = u.unwrap_or_else(|e| {
            if error.is_none() { error = Some(format!("USDS: {}", e)); }
            0
        });
        let susds_raw = s.unwrap_or_else(|e| {
            if error.is_none() { error = Some(format!("sUSDS: {}", e)); }
            0
        });
        let dai_raw = match dai_res {
            Some(Ok(v)) => v,
            Some(Err(e)) => {
                if error.is_none() { error = Some(format!("DAI: {}", e)); }
                0
            }
            None => 0,
        };

        ChainSnapshot { chain, native_raw, usds_raw, susds_raw, dai_raw, error }
    }).collect();
    futures::future::join_all(futs).await
}
