use anyhow::Result;
use reqwest::Client;

use crate::api::check_clob_access;

/// Check whether Polymarket is accessible from the current IP.
/// Run this before topping up USDC.e to confirm your region is not restricted.
pub async fn run() -> Result<()> {
    let client = Client::new();

    let result = match check_clob_access(&client).await {
        Some(warning) => serde_json::json!({
            "ok": true,
            "data": {
                "accessible": false,
                "warning": warning
            }
        }),
        None => serde_json::json!({
            "ok": true,
            "data": {
                "accessible": true,
                "note": "Polymarket is accessible from your current IP. You may proceed to top up USDC.e and trade."
            }
        }),
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
