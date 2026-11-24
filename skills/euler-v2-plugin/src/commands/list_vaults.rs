/// `euler-v2-plugin list-vaults` — list EVK vaults on the requested chain.
///
/// Read-only. Pure API call to `/api/vaults?chainId=<id>`. No wallet, no RPC.

use anyhow::Result;
use clap::Args;

use crate::config::{chain_name, is_supported_chain};

#[derive(Args)]
pub struct ListVaultsArgs {
    #[arg(long, default_value_t = 1)]
    pub chain: u64,

    /// Filter to only verified vaults (default: true).
    #[arg(long, default_value_t = true)]
    pub verified_only: bool,

    /// Limit number of vaults shown (default: 20, max 200).
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

pub async fn run(args: ListVaultsArgs) -> Result<()> {
    match run_inner(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("{}", super::error_response(&e, Some("list-vaults"), None));
            Ok(())
        }
    }
}

async fn run_inner(args: ListVaultsArgs) -> Result<()> {
    if !is_supported_chain(args.chain) {
        anyhow::bail!(
            "Chain {} not supported in v0.1. Use 1 (Ethereum), 8453 (Base), or 42161 (Arbitrum).",
            args.chain
        );
    }
    let limit = args.limit.min(200);

    let vaults = crate::api::get_vaults_raw(args.chain).await?;
    let evk = vaults["evkVaults"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Euler API returned no evkVaults field for chain {}", args.chain))?;

    let mut entries: Vec<serde_json::Value> = evk.iter()
        .filter(|v| !args.verified_only || v["verified"].as_bool() == Some(true))
        .take(limit)
        .map(|v| serde_json::json!({
            "address":     v["address"],
            "name":        v["name"],
            "verified":    v["verified"],
            // Big-int fields are wrapped as {"__bi": "<digits>"} by the API.
            // We surface them as raw decimal strings for now; later commands can
            // enrich with token decimals + USD pricing.
            "supply_raw":  v["supply"]["__bi"].as_str().unwrap_or("0"),
            "borrow_raw":  v["borrow"]["__bi"].as_str().unwrap_or("0"),
            "asset":       v.get("asset"),
            "irm":         v.get("irm"),
        }))
        .collect();
    let total_evk = evk.len();
    let returned = entries.len();

    // Stable order: keep API order (Euler returns by TVL).
    // Truncate over-eager fields if any vault is missing them.
    for e in entries.iter_mut() {
        if e.get("asset").is_none() { e["asset"] = serde_json::Value::Null; }
        if e.get("irm").is_none()   { e["irm"] = serde_json::Value::Null; }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "chain": chain_name(args.chain),
                "chain_id": args.chain,
                "total_evk_vaults": total_evk,
                "returned": returned,
                "verified_only": args.verified_only,
                "vaults": entries,
                "tip": "Use `get-vault --address <address>` for full vault details (APY, caps, oracle). \
                        Use `supply --vault <address> --amount <N>` to deposit."
            }
        }))?
    );
    Ok(())
}
