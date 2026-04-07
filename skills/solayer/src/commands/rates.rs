use crate::config::SOLAYER_API_BASE;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct InfoResponse {
    apy: f64,
    ssol_to_sol: f64,
    tvl_sol: Option<String>,
    tvl_usd: Option<String>,
    epoch: Option<u64>,
    epoch_diff_time: Option<String>,
    ssol_holders: Option<u64>,
    depositors: Option<u64>,
}

pub async fn execute() -> anyhow::Result<Value> {
    let url = format!("{}/info", SOLAYER_API_BASE);
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch Solayer info: {}", e))?;

    if !resp.status().is_success() {
        anyhow::bail!("Solayer API error: HTTP {}", resp.status());
    }

    let info: InfoResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse Solayer info response: {}", e))?;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "apy_percent": info.apy,
            "ssol_to_sol": info.ssol_to_sol,
            "sol_to_ssol": 1.0 / info.ssol_to_sol,
            "tvl_sol": info.tvl_sol.unwrap_or_default(),
            "tvl_usd": info.tvl_usd.unwrap_or_default(),
            "epoch": info.epoch.unwrap_or(0),
            "epoch_remaining": info.epoch_diff_time.unwrap_or_default(),
            "ssol_holders": info.ssol_holders.unwrap_or(0),
            "depositors": info.depositors.unwrap_or(0)
        }
    });
    Ok(result)
}
