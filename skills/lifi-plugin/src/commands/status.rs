use clap::Args;
use serde_json::{json, Value};

use crate::api;
use crate::config::parse_chain;

#[derive(Args)]
pub struct StatusArgs {
    /// Source-chain transaction hash returned by `bridge`
    #[arg(long = "tx-hash")]
    pub tx_hash: String,
    /// Source chain (id or key) — optional but recommended
    #[arg(long)]
    pub from_chain: Option<String>,
    /// Destination chain (id or key) — optional but recommended
    #[arg(long)]
    pub to_chain: Option<String>,
    /// Bridge tool key (e.g. `relay`, `across`, `stargate`) — optional
    #[arg(long)]
    pub bridge: Option<String>,
}

pub async fn run(args: StatusArgs) -> anyhow::Result<()> {
    if !args.tx_hash.starts_with("0x") || args.tx_hash.len() < 10 {
        println!("{}", super::error_response(
            &format!("--tx-hash '{}' does not look like a 0x-prefixed hash", args.tx_hash),
            "INVALID_ARGUMENT",
            "Pass the source-chain tx hash returned by `bridge`.",
        ));
        return Ok(());
    }

    let from_id = match args.from_chain.as_deref().map(parse_chain) {
        Some(Some(c)) => Some(c.id),
        Some(None) => {
            println!("{}", super::error_response(
                &format!("Unknown --from-chain '{}'", args.from_chain.as_deref().unwrap_or("")),
                "UNSUPPORTED_CHAIN",
                "Use a chain in our 6-chain whitelist (ETH/ARB/BAS/OPT/BSC/POL) or pass a numeric id.",
            ));
            return Ok(());
        }
        None => None,
    };
    let to_id = match args.to_chain.as_deref().map(parse_chain) {
        Some(Some(c)) => Some(c.id),
        Some(None) => {
            println!("{}", super::error_response(
                &format!("Unknown --to-chain '{}'", args.to_chain.as_deref().unwrap_or("")),
                "UNSUPPORTED_CHAIN",
                "Use a chain in our 6-chain whitelist or pass a numeric id.",
            ));
            return Ok(());
        }
        None => None,
    };

    let resp = match api::get_status(&args.tx_hash, from_id, to_id, args.bridge.as_deref()).await {
        Ok(v) => v,
        Err(e) => {
            let msg = format!("{:#}", e);
            let code = if msg.contains("404") {
                "STATUS_NOT_FOUND"
            } else if msg.contains("400") {
                "INVALID_STATUS_REQUEST"
            } else {
                "API_ERROR"
            };
            println!("{}", super::error_response(
                &msg, code,
                "Pass --from-chain and --to-chain to disambiguate. NOT_FOUND can also mean tx not yet indexed.",
            ));
            return Ok(());
        }
    };

    let status_str = resp["status"].as_str().unwrap_or("UNKNOWN").to_string();
    let substatus_str = resp.get("substatus").and_then(|v| v.as_str()).map(|s| s.to_string());

    println!("{}", serde_json::to_string_pretty(&json!({
        "ok": true,
        "tx_hash": args.tx_hash,
        "status": status_str,
        "substatus": substatus_str,
        "substatus_message": resp.get("substatusMessage").cloned().unwrap_or(Value::Null),
        "tool": resp.get("tool").cloned().unwrap_or(Value::Null),
        "sending": summarize_leg(&resp["sending"]),
        "receiving": summarize_leg(&resp["receiving"]),
        "lifi_explorer": resp.get("lifiExplorerLink").cloned().unwrap_or(Value::Null),
        "transaction_id": resp.get("transactionId").cloned().unwrap_or(Value::Null),
        "fee_costs": resp.get("feeCosts").cloned().unwrap_or(Value::Array(vec![])),
        "is_terminal": is_terminal(&status_str),
    }))?);
    Ok(())
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "DONE" | "FAILED" | "INVALID")
}

fn summarize_leg(leg: &Value) -> Value {
    if !leg.is_object() {
        return Value::Null;
    }
    json!({
        "tx_hash": leg.get("txHash").cloned().unwrap_or(Value::Null),
        "tx_link": leg.get("txLink").cloned().unwrap_or(Value::Null),
        "chain_id": leg.get("chainId").cloned().unwrap_or(Value::Null),
        "amount": leg.get("amount").cloned().unwrap_or(Value::Null),
        "token": leg.get("token").and_then(|t| t.get("symbol")).cloned().unwrap_or(Value::Null),
        "timestamp": leg.get("timestamp").cloned().unwrap_or(Value::Null),
        "value": leg.get("value").cloned().unwrap_or(Value::Null),
    })
}
