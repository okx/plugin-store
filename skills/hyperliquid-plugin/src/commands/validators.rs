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

        // total_stake: HL returns `stake` as a JSON NUMBER in atomic units (1 HYPE = 1e8)
        let total_stake_u: u64 = v["stake"].as_u64().unwrap_or_default();
        let total_stake_raw = total_stake_u.to_string();
        let total_stake = api::format_hype_amount(total_stake_u);

        // apr: HL has no top-level `apr`. `stats` is [["day",{predictedApr}],["week",{...}],["month",{...}]].
        // Prefer the "month" entry's predictedApr; fall back to the first entry.
        let stats = v["stats"].as_array();
        let apr = stats
            .and_then(|arr| {
                arr.iter()
                    .find(|s| s.get(0).and_then(|k| k.as_str()) == Some("month"))
                    .or_else(|| arr.first())
            })
            .and_then(|entry| entry.get(1))
            .map(|obj| &obj["predictedApr"])
            .map(|p| {
                // predictedApr may be a JSON string or number depending on HL response
                p.as_str()
                    .map(|s| s.to_string())
                    .or_else(|| p.as_f64().map(|n| n.to_string()))
                    .unwrap_or_else(|| "0".to_string())
            })
            .unwrap_or_else(|| "0".to_string());
        // commission may be a JSON string or a number depending on HL response — handle both
        let commission = v.get("commission")
            .filter(|p| !p.is_null())
            .or_else(|| v.get("commissionBps").filter(|p| !p.is_null()))
            .map(|p| p.as_str()
                .map(|s| s.to_string())
                .or_else(|| p.as_f64().map(|n| n.to_string()))
                .unwrap_or_else(|| "0".to_string()))
            .unwrap_or_else(|| "0".to_string());
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
