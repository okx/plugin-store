use clap::Args;
use crate::api;
use crate::config::info_url;
use super::error_response;

#[derive(Args)]
pub struct ValidatorsArgs {
    /// Optional wallet address (not used for this read command, accepted for consistency)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(_args: ValidatorsArgs) -> anyhow::Result<()> {
    let url = info_url();
    let raw = match api::get_validator_summaries(url).await {
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
    let validators = raw.as_array().unwrap_or(&empty);

    let formatted: Vec<serde_json::Value> = validators.iter().map(|v| {
        let validator_address = v["validator"].as_str()
            .or_else(|| v["address"].as_str())
            .unwrap_or("")
            .to_string();
        let name = v["name"].as_str().unwrap_or("").to_string();

        // total_stake: prefer raw string field, fall back to numeric
        let total_stake_raw = v["totalStake"].as_str()
            .map(|s| s.to_string())
            .or_else(|| v["stake"].as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "0".to_string());
        let total_stake_u: u64 = total_stake_raw.parse().unwrap_or_default();
        let total_stake = api::format_hype_amount(total_stake_u);

        let apr = v["apr"].as_str()
            .or_else(|| v["expectedApr"].as_str())
            .unwrap_or("0")
            .to_string();
        let commission = v["commission"].as_str()
            .or_else(|| v["commissionBps"].as_str())
            .unwrap_or("0")
            .to_string();
        let jailed = v["jailed"].as_bool().unwrap_or(false);

        serde_json::json!({
            "validator_address": validator_address,
            "name": name,
            "total_stake": total_stake,
            "total_stake_raw": total_stake_raw,
            "apr": apr,
            "commission": commission,
            "jailed": jailed,
        })
    }).collect();

    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "validator_count": formatted.len(),
        "validators": formatted,
    }))?);
    Ok(())
}
