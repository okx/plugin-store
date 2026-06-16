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
        // HL shape: {time, hash, delta: { <eventType>: {validator, amount, isUndelegate?} }}
        // The `delta` object has a single key naming the event type; its value carries the fields.
        let timestamp = e["time"].as_u64()
            .or_else(|| e["timestamp"].as_u64())
            .unwrap_or_default();

        let (event_type, inner) = e["delta"].as_object()
            .and_then(|m| m.iter().next())
            .map(|(k, v)| (k.clone(), v.clone()))
            .unwrap_or_else(|| ("unknown".to_string(), serde_json::Value::Null));

        // amount is a DECIMAL HYPE STRING (e.g. "0.17")
        let amount_str = inner["amount"].as_str().unwrap_or("0").to_string();
        let amount_raw: u64 = api::parse_hype_amount(&amount_str).unwrap_or_default();
        let validator = inner["validator"].as_str().unwrap_or("").to_string();
        let is_undelegate = inner["isUndelegate"].as_bool();

        let mut event = serde_json::json!({
            "event_type": event_type,
            "amount": api::format_hype_amount(amount_raw),
            "amount_raw": amount_raw.to_string(),
            "validator": validator,
            "timestamp": timestamp,
        });
        if let Some(u) = is_undelegate {
            event["is_undelegate"] = serde_json::json!(u);
        }
        event
    }).collect();

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "event_count": events.len(),
        "events": events,
        "wallet": wallet,
    }))?);
    Ok(())
}
