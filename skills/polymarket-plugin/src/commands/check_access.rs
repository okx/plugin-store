use anyhow::Result;
use reqwest::Client;

use crate::readiness::{assess_readiness_fresh, RegionStatus};

/// Check whether Polymarket is accessible from the current IP.
/// Run this before topping up USDC to confirm your region is not restricted.
pub async fn run() -> Result<()> {
    let client = Client::new();
    let region = assess_readiness_fresh(&client).await;

    let result = match &region {
        RegionStatus::Accessible => serde_json::json!({
            "ok": true,
            "data": {
                "accessible": true,
                "note": "Polymarket is accessible from your current IP. You may proceed to top up USDC and trade."
            }
        }),
        RegionStatus::Restricted { country } => serde_json::json!({
            "ok": true,
            "data": {
                "accessible": false,
                "country": country,
                "warning": format!(
                    "Polymarket is not available from your region ({}) — trading is restricted.",
                    country
                )
            }
        }),
        RegionStatus::Indeterminate { reason } => serde_json::json!({
            "ok": true,
            "data": {
                "accessible": null,
                "indeterminate": true,
                "reason": reason,
                "warning": "Could not determine region status via the Polymarket geoblock endpoint. \
                            Check your connection and run check-access again, or proceed with caution."
            }
        }),
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
