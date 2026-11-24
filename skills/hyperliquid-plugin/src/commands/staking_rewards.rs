use clap::Args;
use crate::api;
use crate::config::info_url;
use crate::onchainos::resolve_wallet;
use crate::config::ARBITRUM_CHAIN_ID;
use super::error_response;

#[derive(Args)]
pub struct StakingRewardsArgs {
    /// Wallet address to query (defaults to the connected onchainos wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: StakingRewardsArgs) -> anyhow::Result<()> {
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
    let raw = match api::get_delegator_rewards(url, &wallet).await {
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

    // Parse rewards: HL returns either a scalar totalRewards or an array per validator
    let total_raw_str = raw["pendingRewards"].as_str()
        .or_else(|| raw["totalRewards"].as_str())
        .unwrap_or("0");
    let total_raw: u64 = total_raw_str.parse().unwrap_or_default();

    if total_raw == 0 {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "no_rewards": true,
            "total_rewards": "0.00000000",
            "total_rewards_raw": "0",
            "rewards": [],
            "wallet": wallet,
        }))?);
        return Ok(());
    }

    // Parse per-validator rewards if available
    let empty = vec![];
    let per_validator = raw["rewards"].as_array().unwrap_or(&empty);
    let rewards: Vec<serde_json::Value> = per_validator.iter().map(|r| {
        let validator = r["validator"].as_str().unwrap_or("").to_string();
        let amt_raw_str = r["amount"].as_str().unwrap_or("0").to_string();
        let amt_raw: u64 = amt_raw_str.parse().unwrap_or_default();
        serde_json::json!({
            "validator": validator,
            "rewards": api::format_hype_amount(amt_raw),
            "rewards_raw": amt_raw_str,
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "no_rewards": false,
        "total_rewards": api::format_hype_amount(total_raw),
        "total_rewards_raw": total_raw_str,
        "rewards": rewards,
        "wallet": wallet,
    }))?);
    Ok(())
}
