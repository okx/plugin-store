/// `euler-v2-plugin positions` — user's positions across all EVK vaults on a chain.
///
/// **v0.2 optimization**: scans all verified vaults in **one Multicall3 RPC call**
/// instead of N parallel `eth_call` round-trips. For Ethereum's 129 verified vaults,
/// that's 1 multicall (~300ms) vs 258 individual eth_calls (~3s).
///
/// Pattern per vault: `(balanceOf(user), debtOf(user))`. Vaults with shares > 0 also
/// get a `previewRedeem(shares)` follow-up call to surface the underlying-asset value.

use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{chain_name, is_supported_chain};
use crate::multicall::{aggregate3, Call3};
use crate::rpc::{build_address_call, build_preview_redeem,
    SELECTOR_BALANCE_OF, SELECTOR_DEBT_OF};

#[derive(Args)]
pub struct PositionsArgs {
    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    /// Wallet address (defaults to active onchainos wallet).
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: PositionsArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("positions"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: PositionsArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!("Chain {} not supported in v0.1. Use 1 / 8453 / 42161.", args.chain);
    }
    let wallet = match args.address {
        Some(a) => a.to_lowercase(),
        None    => crate::onchainos::get_wallet_address(args.chain).await?.to_lowercase(),
    };

    let vaults_raw = crate::api::get_vaults_raw(args.chain).await?;
    let evk = vaults_raw["evkVaults"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Euler API returned no evkVaults"))?;

    // Collect verified vaults with their asset metadata for output enrichment.
    let candidates: Vec<(String, Value)> = evk.iter()
        .filter(|v| v["verified"].as_bool() == Some(true))
        .filter_map(|v| {
            v["address"].as_str()
                .map(|a| (a.to_lowercase(), v.get("asset").cloned().unwrap_or(Value::Null)))
        })
        .collect();
    let total_scanned = candidates.len();

    // ── Phase 1: bundle balanceOf + debtOf for every vault into one multicall ──
    let bal_calldata  = build_address_call(SELECTOR_BALANCE_OF, &wallet);
    let debt_calldata = build_address_call(SELECTOR_DEBT_OF,    &wallet);
    let mut calls: Vec<Call3> = Vec::with_capacity(candidates.len() * 2);
    for (addr, _) in &candidates {
        calls.push(Call3 { target: addr.clone(), allow_failure: true, calldata: bal_calldata.clone() });
        calls.push(Call3 { target: addr.clone(), allow_failure: true, calldata: debt_calldata.clone() });
    }
    let rs = aggregate3(args.chain, &calls).await?;

    // ── Phase 2: for each vault with shares > 0, batch previewRedeem in a second multicall ──
    let mut hit_indexes: Vec<usize> = Vec::new();
    let mut shares_map:  Vec<u128> = vec![0u128; candidates.len()];
    let mut debt_map:    Vec<u128> = vec![0u128; candidates.len()];
    for (i, _) in candidates.iter().enumerate() {
        let shares = rs.get(i * 2).and_then(|r| r.as_u128()).unwrap_or(0);
        let debt   = rs.get(i * 2 + 1).and_then(|r| r.as_u128()).unwrap_or(0);
        shares_map[i] = shares;
        debt_map[i]   = debt;
        if shares > 0 || debt > 0 { hit_indexes.push(i); }
    }

    let preview_calls: Vec<Call3> = hit_indexes.iter()
        .filter(|&&i| shares_map[i] > 0)
        .map(|&i| {
            let addr = &candidates[i].0;
            Call3 { target: addr.clone(), allow_failure: true, calldata: build_preview_redeem(shares_map[i]) }
        })
        .collect();
    let preview_rs = if preview_calls.is_empty() {
        Vec::new()
    } else {
        aggregate3(args.chain, &preview_calls).await?
    };

    // Map preview results back to vault indices
    let mut preview_iter = preview_rs.into_iter();
    let mut hits: Vec<Value> = Vec::with_capacity(hit_indexes.len());
    for &i in &hit_indexes {
        let (addr, asset) = &candidates[i];
        let shares = shares_map[i];
        let debt   = debt_map[i];
        let assets_underlying: Option<u128> = if shares > 0 {
            preview_iter.next().and_then(|r| r.as_u128())
        } else { None };
        hits.push(serde_json::json!({
            "vault":      addr,
            "asset":      asset,
            "shares_raw": shares.to_string(),
            "assets_raw": assets_underlying.map(|a| a.to_string()),
            "debt_raw":   debt.to_string(),
            "has_supply": shares > 0,
            "has_borrow": debt > 0,
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "wallet":        wallet,
                "chain":         chain_name(args.chain),
                "chain_id":      args.chain,
                "vaults_scanned": total_scanned,
                "open_positions": hits.len(),
                "positions":     hits,
                "rpc_calls":     1 + (if preview_calls.is_empty() { 0 } else { 1 }),
                "tip":           "Use `health-factor` to see liquidation buffer if you have borrow positions."
            }
        }))?
    );
    Ok(())
}
