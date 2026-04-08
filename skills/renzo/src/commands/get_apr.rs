use crate::config;
use serde::Deserialize;

#[derive(Deserialize)]
struct AprResponse {
    apr: f64,
}

pub async fn run() -> anyhow::Result<()> {
    // API endpoint: https://app.renzoprotocol.com/api/apr
    let url = format!("{}/apr", config::API_BASE_URL);
    let client = reqwest::Client::new();
    let resp: AprResponse = client.get(&url).send().await?.json().await?;

    println!("{}", serde_json::json!({
        "ok": true,
        "data": {
            "apr_percent": resp.apr,
            "apr_display": format!("{:.4}%", resp.apr),
            "description": "Renzo ezETH restaking APR (annualized, EigenLayer + AVS rewards)"
        }
    }));

    Ok(())
}
