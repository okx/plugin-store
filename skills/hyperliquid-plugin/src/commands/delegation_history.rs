use clap::Args;
use crate::api;
use crate::config::info_url;
use crate::onchainos::resolve_wallet;
use crate::config::ARBITRUM_CHAIN_ID;
use super::error_response;

#[derive(Args)]
pub struct DelegationHistoryArgs {
    /// Wallet address to query (defaults to the connected onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,

    /// Maximum number of events to return (default: all)
    #[arg(long)]
    pub limit: Option<usize>,
}

pub async fn run(args: DelegationHistoryArgs) -> anyhow::Result<()> {
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
    let raw = match api::get_delegation_history(url, &wallet, args.limit).await {
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
    let history = raw.as_array().unwrap_or(&empty);

    let events: Vec<serde_json::Value> = history.iter().map(|e| {
        let event_type = e["type"].as_str()
            .or_else(|| e["action"].as_str())
            .unwrap_or("unknown")
            .to_string();
        let amount_raw_str = e["amount"].as_str()
            .or_else(|| e["delta"].as_str())
            .unwrap_or("0")
            .to_string();
        let amount_raw: u64 = amount_raw_str.parse().unwrap_or_default();
        let timestamp = e["time"].as_u64()
            .or_else(|| e["timestamp"].as_u64())
            .unwrap_or_default();
        let validator = e["validator"].as_str().unwrap_or("").to_string();
        serde_json::json!({
            "event_type": event_type,
            "amount": api::format_hype_amount(amount_raw),
            "amount_raw": amount_raw_str,
            "validator": validator,
            "timestamp": timestamp,
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "event_count": events.len(),
        "events": events,
        "wallet": wallet,
    }))?);
    Ok(())
}
