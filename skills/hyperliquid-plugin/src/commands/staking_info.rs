use clap::Args;
use crate::api;
use crate::config::info_url;
use crate::onchainos::resolve_wallet;
use crate::config::ARBITRUM_CHAIN_ID;
use super::error_response;

#[derive(Args)]
pub struct StakingInfoArgs {
    /// Wallet address to query (defaults to the connected onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: StakingInfoArgs) -> anyhow::Result<()> {
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
    let delegations = match api::get_delegations(url, &wallet).await {
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
    let delegation_arr = delegations.as_array().unwrap_or(&empty);

    if delegation_arr.is_empty() {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "has_stake": false,
            "staked_amount": "0.00000000",
            "staked_amount_raw": "0",
            "pending_rewards": "0.00000000",
            "pending_rewards_raw": "0",
            "delegations": [],
            "wallet": wallet,
        }))?);
        return Ok(());
    }

    // Aggregate staked_amount across all delegations
    let mut total_staked_raw: u64 = 0;
    let formatted_delegations: Vec<serde_json::Value> = delegation_arr.iter().map(|d| {
        let validator = d["validator"].as_str().unwrap_or("").to_string();
        // amount is a DECIMAL HYPE STRING (e.g. "0.17")
        let amount_str = d["amount"].as_str().unwrap_or("0").to_string();
        let amount_raw: u64 = api::parse_hype_amount(&amount_str).unwrap_or_default();
        total_staked_raw += amount_raw;
        let mut entry = serde_json::json!({
            "validator": validator,
            "staked_amount": api::format_hype_amount(amount_raw),
            "staked_amount_raw": amount_raw.to_string(),
        });
        // lockedUntil: present on HL delegation entries when funds are lock-protected
        if let Some(locked_until) = d["lockedUntilTimestamp"].as_u64().or_else(|| d["lockedUntil"].as_u64()) {
            entry["locked_until"] = serde_json::json!(locked_until);
        }
        entry
    }).collect();

    // Fetch pending rewards
    let rewards_raw: u64;
    let rewards_raw_str: String;
    match api::get_delegator_rewards(url, &wallet).await {
        Ok(r) => {
            // rewards amounts are DECIMAL HYPE STRINGS (e.g. "0.17")
            let reward_str = r["pendingRewards"].as_str()
                .or_else(|| r["totalRewards"].as_str())
                .unwrap_or("0");
            rewards_raw = api::parse_hype_amount(reward_str).unwrap_or_default();
            rewards_raw_str = rewards_raw.to_string();
        }
        Err(_) => {
            rewards_raw = 0;
            rewards_raw_str = "0".to_string();
        }
    }

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "has_stake": true,
        "staked_amount": api::format_hype_amount(total_staked_raw),
        "staked_amount_raw": total_staked_raw.to_string(),
        "pending_rewards": api::format_hype_amount(rewards_raw),
        "pending_rewards_raw": rewards_raw_str,
        "delegations": formatted_delegations,
        "wallet": wallet,
    }))?);
    Ok(())
}
