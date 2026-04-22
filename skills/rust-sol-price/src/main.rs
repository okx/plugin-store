use serde::Deserialize;

#[derive(Deserialize)]
struct OkxResponse { data: Vec<Ticker> }

#[derive(Deserialize)]
struct Ticker {
    last: String,
    #[serde(rename = "open24h")] open_24h: String,
    #[serde(rename = "vol24h")] vol_24h: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let resp: OkxResponse = reqwest::Client::new()
        .get("https://www.okx.com/api/v5/market/ticker?instId=SOL-USDT")
        .send().await?.json().await?;

    let t = resp.data.first().ok_or("No data")?;
    let price: f64 = t.last.parse()?;
    let open: f64 = t.open_24h.parse()?;

    let out = serde_json::json!({
        "token": "SOL",
        "price_usd": format!("{:.2}", price),
        "change_24h": format!("{:+.2}%", ((price - open) / open) * 100.0),
        "volume_24h": t.vol_24h,
        "timestamp": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
