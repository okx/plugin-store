/// `euler-v2-plugin quickstart` — onboarding entry point.
///
/// In a single call (≤ 5 seconds, parallel API + RPC):
///   1. Resolve the user's wallet address on the requested chain
///   2. Fetch Euler chain address book (lens contracts, etc.)
///   3. Fetch the user's positions across vaults (parallel: read-only API)
///   4. Compute a `status` enum and a ready-to-run `next_command`
///
/// Output (always structured JSON, exit 0):
///   {
///     "ok": true,
///     "data": {
///       "wallet": "0x...",
///       "chain": "ethereum",
///       "chain_id": 1,
///       "status": "<enum>",
///       "next_command": "<command-string>",
///       "tip": "<human-readable next step>",
///       "vault_count": N,             // total vaults available on this chain
///       "open_positions": N,          // user's open positions
///       "supply_value_usd": <n>,
///       "borrow_value_usd": <n>,
///       "health_factor": <n>          // null if no borrow
///     }
///   }

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain, SUPPORTED_CHAINS};

/// Status enum — every value has a corresponding step in SUMMARY.md's Quick Start.
const STATUS_NO_FUNDS:        &str = "no_funds";
const STATUS_LOW_BALANCE:     &str = "low_balance";
const STATUS_READY_TO_SUPPLY: &str = "ready_to_supply";
const STATUS_ACTIVE:          &str = "active";
const STATUS_AT_RISK:         &str = "at_risk";
const STATUS_LIQUIDATABLE:    &str = "liquidatable";
const STATUS_CHAIN_INVALID:   &str = "chain_invalid";

#[derive(Args)]
pub struct QuickstartArgs {
    /// Chain ID to check (default: 1 / Ethereum). v0.1 supports 1, 8453, 42161.
    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    /// Wallet address override (defaults to active onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: QuickstartArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("quickstart"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: QuickstartArgs) -> Result<()> {
    // 1. Validate chain up front
    if !is_supported_chain(args.chain) {
        let supported: Vec<String> = SUPPORTED_CHAINS.iter()
            .map(|(id, n)| format!("{} ({})", id, n))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "data": {
                    "status": STATUS_CHAIN_INVALID,
                    "chain_id": args.chain,
                    "next_command": format!("euler-v2-plugin quickstart --chain 1"),
                    "tip": format!("Chain {} is not supported in v0.1. Re-run with one of: {}", args.chain, supported.join(", ")),
                    "supported_chains": supported,
                }
            }))?
        );
        return Ok(());
    }

    // 2. Resolve wallet
    let wallet = match args.address {
        Some(a) => a,
        None    => crate::onchainos::get_wallet_address(args.chain).await?,
    };

    // 3. Parallel: fetch chain address book + vault list
    let chain_fut  = crate::api::get_chain(args.chain);
    let vaults_fut = crate::api::get_vaults_raw(args.chain);
    let (chain_res, vaults_res) = tokio::join!(chain_fut, vaults_fut);

    let _chain_info = chain_res?;  // for now we just verify it loads; later commands use the address book
    let vaults = vaults_res?;

    let vault_count = vaults["evkVaults"].as_array().map(|a| a.len()).unwrap_or(0);

    // 4. Scan user positions on-chain (parallel balanceOf + debtOf across verified vaults).
    let evk = vaults["evkVaults"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Euler API returned no evkVaults"))?;
    let candidates: Vec<String> = evk.iter()
        .filter(|v| v["verified"].as_bool() == Some(true))
        .filter_map(|v| v["address"].as_str().map(|a| a.to_lowercase()))
        .collect();

    use crate::rpc::{build_address_call, eth_call, parse_uint256_to_u128, SELECTOR_BALANCE_OF, SELECTOR_DEBT_OF};
    const RPC_CONCURRENCY: usize = 10;
    let mut open_positions: u64 = 0;
    let mut has_borrow = false;
    let mut has_supply = false;
    let mut vault_rpc_failures: u64 = 0;  // EVM-012: track per-vault read failures
    for chunk in candidates.chunks(RPC_CONCURRENCY) {
        let futs = chunk.iter().map(|addr| {
            let chain = args.chain;
            let wallet_l = wallet.to_lowercase();
            let addr = addr.clone();
            async move {
                let bal_call  = build_address_call(SELECTOR_BALANCE_OF, &wallet_l);
                let debt_call = build_address_call(SELECTOR_DEBT_OF, &wallet_l);
                let (bal_res, debt_res) = tokio::join!(
                    eth_call(chain, &addr, &bal_call),
                    eth_call(chain, &addr, &debt_call),
                );
                // EVM-012: silent unwrap_or(0) on per-vault RPC failure used to
                // mis-route quickstart to `no_funds` status when the user
                // actually had positions in vaults that just had transient RPC
                // failures. Track each call's success so we can surface the
                // count to the caller and let them retry with confidence.
                let shares_opt = bal_res.ok().map(|h| parse_uint256_to_u128(&h));
                let debt_opt   = debt_res.ok().map(|h| parse_uint256_to_u128(&h));
                (shares_opt, debt_opt)
            }
        });
        let results: Vec<_> = futures::future::join_all(futs).await;
        for (shares_opt, debt_opt) in results {
            // Count RPC failures (either the balance OR debt read for this
            // vault failed). Don't double-count if both fail.
            if shares_opt.is_none() || debt_opt.is_none() {
                vault_rpc_failures += 1;
            }
            let shares = shares_opt.unwrap_or(0);
            let debt   = debt_opt.unwrap_or(0);
            if shares > 0 || debt > 0 { open_positions += 1; }
            if shares > 0 { has_supply = true; }
            if debt > 0   { has_borrow = true; }
        }
    }
    // v0.1 doesn't compute USD values yet — defer until oracle/price integration.
    let supply_value_usd: f64 = if has_supply { 1.0 } else { 0.0 };
    let borrow_value_usd: f64 = if has_borrow { 1.0 } else { 0.0 };
    let health_factor: Option<f64> = None;

    // 5. Compute status + next_command
    let (status, tip, next_command) = if open_positions == 0 {
        if supply_value_usd < 5.0 {
            (
                STATUS_NO_FUNDS,
                format!(
                    "No Euler positions on {}. To start lending, supply an asset to an EVK vault. \
                     Run `list-vaults` to browse available vaults.",
                    chain_name(args.chain).unwrap_or("this chain"),
                ),
                "euler-v2-plugin list-vaults --chain ".to_string() + &args.chain.to_string(),
            )
        } else {
            (
                STATUS_READY_TO_SUPPLY,
                "You have funds available. Browse vaults and pick one to supply to.".to_string(),
                "euler-v2-plugin list-vaults --chain ".to_string() + &args.chain.to_string(),
            )
        }
    } else if let Some(hf) = health_factor {
        if hf < 1.0 {
            (
                STATUS_LIQUIDATABLE,
                format!("⚠ Health factor {:.3} — liquidation risk active. Repay debt or add collateral immediately.", hf),
                "euler-v2-plugin repay --all".to_string(),
            )
        } else if hf < 1.5 {
            (
                STATUS_AT_RISK,
                format!("Health factor {:.3} — getting close to liquidation. Consider repaying or topping up collateral.", hf),
                "euler-v2-plugin positions --chain ".to_string() + &args.chain.to_string(),
            )
        } else {
            (
                STATUS_ACTIVE,
                format!("You have {} open position(s) with healthy collateralization (HF {:.3}).", open_positions, hf),
                "euler-v2-plugin positions --chain ".to_string() + &args.chain.to_string(),
            )
        }
    } else {
        (
            STATUS_ACTIVE,
            format!("You have {} open supply position(s) on Euler. No active borrows.", open_positions),
            "euler-v2-plugin positions --chain ".to_string() + &args.chain.to_string(),
        )
    };

    let _ = STATUS_LOW_BALANCE;  // reserved for future use when supply_value_usd in (0, 5)

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "about": "Euler v2 — modular lending protocol with isolated-risk EVK vaults. Supply assets to earn yield; borrow against collateral.",
                "wallet": wallet,
                "chain": chain_name(args.chain),
                "chain_id": args.chain,
                "status": status,
                "next_command": next_command,
                "tip": tip,
                "vault_count": vault_count,
                "open_positions": open_positions,
                "vault_rpc_failures": vault_rpc_failures,
                "supply_value_usd": supply_value_usd,
                "borrow_value_usd": borrow_value_usd,
                "health_factor": health_factor,
                "_note": "v0.1: position counting is real (on-chain balanceOf + debtOf scan). \
                          USD values + health_factor still pending — they need oracle / lens contract integration in v0.2."
            }
        }))?
    );
    Ok(())
}
