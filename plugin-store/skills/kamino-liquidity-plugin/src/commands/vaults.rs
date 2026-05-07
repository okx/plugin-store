use clap::Args;
use serde_json::Value;

use crate::api;

#[derive(Args, Debug)]
pub struct VaultsArgs {
    /// Chain ID (must be 501 for Solana)
    #[arg(long, default_value = "501")]
    pub chain: u64,

    /// Filter by token symbol (e.g. SOL, USDC) — case-insensitive substring match on vault name
    #[arg(long)]
    pub token: Option<String>,

    /// Maximum number of vaults to show (default: 20)
    #[arg(long, default_value = "20")]
    pub limit: usize,
}

pub async fn run(args: VaultsArgs) -> anyhow::Result<()> {
    if args.chain != 501 {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": false,
                "error": "kamino-liquidity only supports Solana (chain 501)",
                "error_code": "UNSUPPORTED_CHAIN",
                "suggestion": "Use --chain 501 or omit --chain (defaults to 501)."
            }))?
        );
        return Ok(());
    }

    let raw = match api::get_vaults().await {
        Ok(r) => r,
        Err(e) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": format!("{:#}", e),
                    "error_code": "KAMINO_API_ERROR",
                    "suggestion": "Kamino API request failed. Check your connection and retry."
                }))?
            );
            return Ok(());
        }
    };

    let vaults = match raw.as_array() {
        Some(v) => v,
        None => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "ok": false,
                    "error": format!("Unexpected response from Kamino API: {}", raw),
                    "error_code": "KAMINO_API_ERROR",
                    "suggestion": "Kamino API returned an unexpected format. Retry or check api.kamino.finance status."
                }))?
            );
            return Ok(());
        }
    };

    let mut results: Vec<serde_json::Map<String, Value>> = Vec::new();

    for vault in vaults {
        let address = vault["address"].as_str().unwrap_or("").to_string();
        let state = &vault["state"];
        let name = state["name"].as_str().unwrap_or("").to_string();
        let token_mint = state["tokenMint"].as_str().unwrap_or("").to_string();
        let token_decimals = state["tokenMintDecimals"].as_u64().unwrap_or(6);
        let shares_mint = state["sharesMint"].as_str().unwrap_or("").to_string();
        let shares_issued = state["sharesIssued"].as_str().unwrap_or("0").to_string();
        let token_available = state["tokenAvailable"].as_str().unwrap_or("0").to_string();
        let perf_fee_bps = state["performanceFeeBps"].as_u64().unwrap_or(0);
        let mgmt_fee_bps = state["managementFeeBps"].as_u64().unwrap_or(0);
        let alloc_count = state["vaultAllocationStrategy"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);

        // Filter by token name if requested
        if let Some(ref filter) = args.token {
            if !name.to_lowercase().contains(&filter.to_lowercase())
                && !token_mint.to_lowercase().contains(&filter.to_lowercase())
            {
                continue;
            }
        }

        let mut entry = serde_json::Map::new();
        entry.insert("address".into(), Value::String(address));
        entry.insert("name".into(), Value::String(name));
        entry.insert("token_mint".into(), Value::String(token_mint));
        entry.insert("token_decimals".into(), Value::Number(token_decimals.into()));
        entry.insert("shares_mint".into(), Value::String(shares_mint));
        entry.insert("shares_issued".into(), Value::String(shares_issued));
        entry.insert("token_available".into(), Value::String(token_available));
        entry.insert(
            "performance_fee_bps".into(),
            Value::Number(perf_fee_bps.into()),
        );
        entry.insert(
            "management_fee_bps".into(),
            Value::Number(mgmt_fee_bps.into()),
        );
        entry.insert(
            "allocation_count".into(),
            Value::Number(alloc_count.into()),
        );
        results.push(entry);

        if results.len() >= args.limit {
            break;
        }
    }

    let output = serde_json::json!({
        "ok": true,
        "chain": args.chain,
        "total": vaults.len(),
        "shown": results.len(),
        "vaults": results
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
