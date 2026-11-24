use clap::Args;
use serde_json::json;

use crate::api::{fetch_eth_price, fetch_pufeth_apy};
use crate::config::{format_units, pufeth_address, puffer_vault_address, rpc_url, CHAIN_ID};
use crate::onchainos::{resolve_wallet, wallet_balance};
use crate::rpc::{convert_to_assets, get_balance, get_total_exit_fee_bps};

const ABOUT: &str = "Puffer Finance is a liquid restaking protocol on Ethereum. Deposit ETH to mint pufETH (an ERC-4626 nLRT vault token) and earn restaking yield. Two exit paths: 1-step instant withdraw (1% fee, immediate WETH) or 2-step queued withdraw (~14 days, fee-free).";

/// Minimum ETH (in wei) to consider "fundable" for a stake action.
/// 0.005 ETH (~$11.50) is enough to cover gas + a meaningful stake.
const STAKE_MIN_ETH_WEI: u128 = 5_000_000_000_000_000;

/// Minimum pufETH share count (in wei-equivalent, 18 dec) to be considered "earning".
/// Anything smaller is dust from prior tests.
const PUFETH_DUST_THRESHOLD: u128 = 1_000_000_000_000_000; // 0.001 pufETH

#[derive(Args)]
pub struct QuickstartArgs {
    /// Wallet address to query. Defaults to the connected onchainos wallet.
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: QuickstartArgs) -> anyhow::Result<()> {
    if let Err(e) = run_inner(args).await {
        println!("{}", super::error_response(&e, Some("quickstart")));
    }
    Ok(())
}

async fn run_inner(args: QuickstartArgs) -> anyhow::Result<()> {
    // ── 1. Resolve wallet ─────────────────────────────────────────────────────
    let wallet = match args.address {
        Some(addr) => addr,
        None => resolve_wallet(CHAIN_ID)?,
    };

    eprintln!("Scanning Puffer state on Ethereum for {}...", &wallet[..std::cmp::min(10, wallet.len())]);

    // ── 2. Parallel reads: ETH balance + pufETH balance + rate + price + APY ──
    let rpc = rpc_url();
    let vault = puffer_vault_address();

    let eth_bal_fut = wallet_balance(CHAIN_ID, None, false);
    let pufeth_bal_fut = get_balance(pufeth_address(), &wallet, rpc);
    let rate_fut = convert_to_assets(vault, 1_000_000_000_000_000_000, rpc); // 1 share → assets
    let exit_fee_fut = get_total_exit_fee_bps(vault, rpc);

    // Run parallel; capture errors without short-circuiting (each is best-effort)
    let (eth_bal_res, pufeth_res, rate_res, exit_fee_res) =
        tokio::join!(eth_bal_fut, pufeth_bal_fut, rate_fut, exit_fee_fut);

    // External: best-effort, never fails the whole call
    let (eth_price_opt, apy_opt) = tokio::join!(fetch_eth_price(), fetch_pufeth_apy());

    // ── 3. Tally RPC failures (EVM-012-style: don't pretend 0 is real) ────────
    let mut rpc_failures = 0;
    let eth_bal_wei = match eth_bal_res {
        Ok(v) => v,
        Err(_) => { rpc_failures += 1; 0 }
    };
    let pufeth_raw = match pufeth_res {
        Ok(v) => v,
        Err(_) => { rpc_failures += 1; 0 }
    };
    let one_share_assets = match rate_res {
        Ok(v) => v,
        Err(_) => { rpc_failures += 1; 0 }
    };
    let exit_fee_bps = exit_fee_res.unwrap_or(100); // default 1% fallback for display only

    // pufETH → ETH equivalent (uses live rate)
    let eth_equiv_raw = if pufeth_raw > 0 && one_share_assets > 0 {
        // shares × (assets per 1 share) / 1e18
        ((pufeth_raw as u128) * (one_share_assets as u128) / 1_000_000_000_000_000_000) as u128
    } else {
        0
    };

    // ── 4. Decide status + next_command ───────────────────────────────────────
    let has_pufeth = pufeth_raw >= PUFETH_DUST_THRESHOLD;
    let has_stakeable_eth = eth_bal_wei >= STAKE_MIN_ETH_WEI;

    let (status, next_command, tip): (&str, Option<String>, String) = if rpc_failures >= 2 {
        ("rpc_degraded", None,
         "More than half the on-chain RPC reads failed. Retry in a minute.".to_string())
    } else if !has_pufeth && !has_stakeable_eth {
        ("no_funds",
         Some(format!("puffer-plugin positions --wallet {}", wallet)),
         format!("Wallet has neither pufETH nor enough ETH to stake (≥0.005 ETH = ~$11). Bridge ETH to Ethereum mainnet first, then stake.")
        )
    } else if has_pufeth {
        // Already earning — show position; if user wants to exit, withdraw-options compares paths
        let amt_human = format_units(pufeth_raw, 18);
        let eth_eq_human = format_units(eth_equiv_raw, 18);
        ("has_pufeth_earning",
         Some(format!("puffer-plugin positions --wallet {}", wallet)),
         format!("You hold {} pufETH (≈ {} ETH @ live rate). To exit, run `puffer-plugin withdraw-options --amount <X>` to compare 1-step instant (1% fee) vs 2-step queued (~14d, no fee).", amt_human, eth_eq_human),
        )
    } else {
        // Has ETH, no pufETH yet — invite to stake
        let suggested_amt = sensible_stake_amount(eth_bal_wei);
        ("ready_to_stake",
         Some(format!("puffer-plugin stake --amount {} --confirm", suggested_amt)),
         format!("You have {} ETH but no pufETH. Stake to start earning ~{}% APY restaking yield.",
            format_units(eth_bal_wei, 18),
            apy_opt.map(|a| format!("{:.2}", a)).unwrap_or_else(|| "?".to_string())
         )
        )
    };

    // ── 5. Render structured output ───────────────────────────────────────────
    let eth_equiv_usd = match (eth_price_opt, eth_equiv_raw) {
        (Some(p), e) if e > 0 => Some(format!("{:.2}", (e as f64 / 1e18) * p)),
        _ => None,
    };

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "about": ABOUT,
        "chain": "Ethereum",
        "chain_id": CHAIN_ID,
        "wallet": wallet,
        "rpc_failures": rpc_failures,
        "current_apy_pct": apy_opt.map(|a| format!("{:.4}", a)),
        "exit_fee_bps": exit_fee_bps,
        "rate_one_pufeth_to_eth": format_units(one_share_assets, 18),
        "balances": {
            "eth": {
                "amount": format_units(eth_bal_wei, 18),
                "amount_raw": eth_bal_wei.to_string(),
            },
            "pufeth": {
                "amount": format_units(pufeth_raw, 18),
                "amount_raw": pufeth_raw.to_string(),
                "eth_equivalent": format_units(eth_equiv_raw, 18),
                "eth_equivalent_raw": eth_equiv_raw.to_string(),
                "usd_equivalent": eth_equiv_usd,
            }
        },
        "status": status,
        "next_command": next_command,
        "tip": tip,
        "note": "Queued (2-step) withdrawals are NOT automatically scanned by quickstart — index-based lookup is expensive. If you have a pending withdrawal index from `request-withdraw`, query directly via `puffer-plugin withdraw-status --index <N>`.",
    }))?);
    Ok(())
}

/// Return a sensible stake amount given ETH balance, leaving ~$5 for gas.
fn sensible_stake_amount(eth_wei: u128) -> String {
    // ~0.002 ETH gas reserve for stake + later withdraw
    let gas_reserve: u128 = 2_000_000_000_000_000; // 0.002 ETH
    let stakable = eth_wei.saturating_sub(gas_reserve);
    if stakable < STAKE_MIN_ETH_WEI {
        return "0.005".to_string();
    }
    // Round down to nearest 0.001 ETH for clean numbers, capped at 0.05 for first-test feel
    let cap: u128 = 50_000_000_000_000_000; // 0.05 ETH
    let pick = stakable.min(cap);
    format_units(pick, 18)
}
