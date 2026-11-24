use clap::Args;
use serde_json::{json, Value};

use crate::api;
use crate::config::SUPPORTED_CHAINS;

#[derive(Args)]
pub struct ChainsArgs {
    /// Show all chains LI.FI supports (not just our 6-chain whitelist)
    #[arg(long)]
    pub all: bool,
}

pub async fn run(args: ChainsArgs) -> anyhow::Result<()> {
    if !args.all {
        // Local whitelist — no network call needed for the common case
        let chains: Vec<Value> = SUPPORTED_CHAINS
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "key": c.key,
                    "name": c.name,
                    "native_symbol": c.native_symbol,
                    "rpc": c.rpc,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({
            "ok": true,
            "source": "local_whitelist",
            "count": chains.len(),
            "chains": chains,
        }))?);
        return Ok(());
    }

    // --all: ask LI.FI for the full chain registry
    let resp = match api::get_chains().await {
        Ok(v) => v,
        Err(e) => {
            println!(
                "{}",
                super::error_response(
                    &format!("{:#}", e),
                    "API_ERROR",
                    "LI.FI /v1/chains is unreachable. Check connectivity or retry."
                )
            );
            return Ok(());
        }
    };

    let chain_list = resp["chains"].as_array().cloned().unwrap_or_default();
    let summarized: Vec<Value> = chain_list
        .iter()
        .map(|c| {
            json!({
                "id": c.get("id").cloned().unwrap_or(Value::Null),
                "key": c.get("key").cloned().unwrap_or(Value::Null),
                "name": c.get("name").cloned().unwrap_or(Value::Null),
                "chainType": c.get("chainType").cloned().unwrap_or(Value::Null),
                "nativeToken": c.get("nativeToken").and_then(|t| t.get("symbol")).cloned().unwrap_or(Value::Null),
                "mainnet": c.get("mainnet").cloned().unwrap_or(Value::Bool(true)),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "source": "lifi_api",
        "count": summarized.len(),
        "chains": summarized,
    }))?);
    Ok(())
}
