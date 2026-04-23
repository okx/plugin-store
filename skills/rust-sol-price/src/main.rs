use serde::Deserialize;
#[derive(Deserialize)]
struct R { data: Vec<T> }
#[derive(Deserialize)]
struct T { last: String, #[serde(rename="open24h")] o: String, #[serde(rename="vol24h")] v: String }
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let r: R = reqwest::Client::new().get("https://www.okx.com/api/v5/market/ticker?instId=SOL-USDT").send().await?.json().await?;
    let t = r.data.first().ok_or("No data")?;
    let p: f64 = t.last.parse()?; let o: f64 = t.o.parse()?;
    let out = serde_json::json!({"token":"SOL","price":format!("{:.2}",p),"change":format!("{:+.2}%",((p-o)/o)*100.0),"vol":t.v,"ts":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()});
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
