use clap::Args;
use crate::api;
use crate::config::info_url;
use crate::onchainos::resolve_wallet;
use crate::config::ARBITRUM_CHAIN_ID;
use super::error_response;

#[derive(Args)]
pub struct UnbondingArgs {
    /// Wallet address to query (defaults to the connected onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: UnbondingArgs) -> anyhow::Result<()> {
    let wallet = match args.address {
        Some(addr) => addr,
        None => match resolve_wallet(ARBITRUM_CHAIN_ID) {
            Ok(w) => w,
            Err(e) => {
                println!("{}", error_response(
                    &format!("{:#}", e),
                    "WALLET_NOT_FOUND",
                    "Run onchainos wallet addresses to verify login.",
                ));
                return Ok(());
            }
        },
    };

    let url = info_url();
    // Unbonding state lives in the delegations response — entries with "lockedUntil" or similar
    // HL may return unbonding info via a separate type; we check delegations for unbonding entries
    let raw = match api::get_delegations(url, &wallet).await {
        Ok(v) => v,
        Err(e) => {
            println!("{}", error_response(
                &format!("{:#}", e),
                "API_ERROR",
                "Check your connection and retry.",
            ));
            return Ok(());
        }
    };

    let empty = vec![];
    let all = raw.as_array().unwrap_or(&empty);

    // Filter entries that are in unbonding state (have a lockedUntil / completionTime field)
    let active_unbonds: Vec<serde_json::Value> = all.iter().filter_map(|d| {
        let locked_until = d["lockedUntil"].as_u64()
            .or_else(|| d["completionTime"].as_u64());
        locked_until.map(|ts| {
            let validator = d["validator"].as_str().unwrap_or("").to_string();
            let amount_raw_str = d["amount"].as_str().unwrap_or("0").to_string();
            let amount_raw: u64 = amount_raw_str.parse().unwrap_or_default();
            serde_json::json!({
                "validator": validator,
                "amount": api::format_hype_amount(amount_raw),
                "amount_raw": amount_raw_str,
                "estimated_completion_ts": ts,
            })
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "active_unbonds_count": active_unbonds.len(),
        "active_unbonds": active_unbonds,
        "wallet": wallet,
    }))?);
    Ok(())
}
