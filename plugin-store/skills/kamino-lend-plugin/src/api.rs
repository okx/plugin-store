use anyhow::Result;
use serde_json::Value;

use crate::config::API_BASE;

const JUPITER_API: &str = "https://api.jup.ag/swap/v1";
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const JUPITER_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

/// Swap SOL → output_mint via Jupiter Aggregator.
/// sol_lamports: amount of native SOL to swap (in lamports; 1_000_000 = 0.001 SOL).
/// Returns base64-encoded serialized transaction ready for onchainos.
pub async fn jupiter_swap_sol_to_token(
    wallet: &str,
    output_mint: &str,
    sol_lamports: u64,
) -> Result<String> {
    let client = reqwest::Client::new();

    // Step 1: get quote
    let quote_url = format!(
        "{}/quote?inputMint={}&outputMint={}&amount={}&slippageBps=300",
        JUPITER_API, SOL_MINT, output_mint, sol_lamports
    );
    let quote: Value = client.get(&quote_url).send().await?.json().await?;
    if let Some(err) = quote.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!("Jupiter quote error: {}", err);
    }

    // Step 2: build swap transaction
    let swap_body = serde_json::json!({
        "quoteResponse": quote,
        "userPublicKey": wallet,
        "wrapAndUnwrapSol": true,
        "dynamicComputeUnitLimit": true,
        "prioritizationFeeLamports": "auto"
    });
    let swap_resp: Value = client
        .post(format!("{}/swap", JUPITER_API))
        .json(&swap_body)
        .send()
        .await?
        .json()
        .await?;

    swap_resp["swapTransaction"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Jupiter swap error: {}", swap_resp))
}

/// Fetch all Kamino Lend reserves from DeFiLlama in a single fast call.
/// Filters to project=kamino-lend, chain=Solana.
/// Returns raw DeFiLlama pool objects (fields: symbol, apy, apyBorrow, tvlUsd, …).
pub async fn fetch_kamino_reserves_defillama() -> anyhow::Result<Vec<serde_json::Value>> {
    let url = "https://yields.llama.fi/pools";
    let resp = reqwest::Client::new().get(url).send().await?;
    let data: serde_json::Value = resp.json().await?;
    let all = data["data"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("DeFiLlama returned no data array"))?;

    let kamino: Vec<serde_json::Value> = all
        .iter()
        .filter(|p| {
            let proj = p["project"].as_str().unwrap_or("");
            let chain = p["chain"].as_str().unwrap_or("");
            proj == "kamino-lend" && chain.eq_ignore_ascii_case("solana")
        })
        .cloned()
        .collect();
    Ok(kamino)
}

/// Fetch symbol and decimals for a reserve via the metrics/history endpoint.
/// Returns None if the reserve is not found or the API call fails.
pub async fn get_reserve_info(market: &str, reserve: &str) -> Option<(String, u32)> {
    let end = chrono_approx_now();
    let start = chrono_approx_yesterday();
    let url = format!(
        "{}/kamino-market/{}/reserves/{}/metrics/history?env=mainnet-beta&start={}&end={}&frequency=day",
        API_BASE, market, reserve, start, end
    );
    let resp = reqwest::Client::new().get(&url).send().await.ok()?;
    let data: Value = resp.json().await.ok()?;
    let history = data["history"].as_array()?;
    let latest = history.last()?;
    let metrics = &latest["metrics"];
    let symbol = metrics["symbol"].as_str()?.to_string();
    let decimals = metrics["decimals"].as_u64()? as u32;
    Some((symbol, decimals))
}

/// Fetch the current USD price for a reserve (assetPriceUSD field from metrics).
/// Returns None on any error.
pub async fn get_reserve_price_usd(market: &str, reserve: &str) -> Option<f64> {
    let end = chrono_approx_now();
    let start = chrono_approx_yesterday();
    let url = format!(
        "{}/kamino-market/{}/reserves/{}/metrics/history?env=mainnet-beta&start={}&end={}&frequency=day",
        API_BASE, market, reserve, start, end
    );
    let resp = reqwest::Client::new().get(&url).send().await.ok()?;
    let data: Value = resp.json().await.ok()?;
    let latest = data["history"].as_array()?.last()?;
    let price_val = &latest["metrics"]["assetPriceUSD"];
    // assetPriceUSD can be a JSON string or number depending on API version
    price_val
        .as_f64()
        .or_else(|| price_val.as_str().and_then(|s| s.parse::<f64>().ok()))
}

/// Fetch all Kamino lending markets.
/// GET /v2/kamino-market
pub async fn get_markets() -> Result<Value> {
    let url = format!("{}/v2/kamino-market", API_BASE);
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let data: Value = resp.json().await?;
    Ok(data)
}

/// Fetch reserve metrics history for a single reserve.
/// GET /kamino-market/{market}/reserves/{reserve}/metrics/history
/// Returns the latest snapshot (last 24h, daily frequency).
pub async fn get_reserve_metrics(market: &str, reserve: &str) -> Result<Value> {
    // Use a 2-day window to ensure we get at least one data point
    let end = chrono_approx_now();
    let start = chrono_approx_yesterday();
    let url = format!(
        "{}/kamino-market/{}/reserves/{}/metrics/history?env=mainnet-beta&start={}&end={}&frequency=day",
        API_BASE, market, reserve, start, end
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let data: Value = resp.json().await?;
    Ok(data)
}

/// Fetch user obligations (positions) in a market.
/// GET /kamino-market/{market}/users/{wallet}/obligations
pub async fn get_obligations(market: &str, wallet: &str) -> Result<Value> {
    let url = format!(
        "{}/kamino-market/{}/users/{}/obligations",
        API_BASE, market, wallet
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let data: Value = resp.json().await?;
    Ok(data)
}

/// Build a deposit (supply) transaction.
/// POST /ktx/klend/deposit
/// Returns: { "transaction": "<base64_serialized_tx>" }
/// Amount: UI units (e.g., "0.01" for 0.01 USDC)
pub async fn build_deposit_tx(
    wallet: &str,
    market: &str,
    reserve: &str,
    amount: &str,
) -> Result<String> {
    let url = format!("{}/ktx/klend/deposit", API_BASE);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "wallet": wallet,
        "market": market,
        "reserve": reserve,
        "amount": amount
    });
    let resp = client.post(&url).json(&body).send().await?;
    let data: Value = resp.json().await?;
    if let Some(tx) = data["transaction"].as_str() {
        Ok(tx.to_string())
    } else {
        anyhow::bail!(
            "Kamino API deposit error: {}",
            data["message"].as_str().unwrap_or("unknown error")
        )
    }
}

/// Build a withdraw transaction.
/// POST /ktx/klend/withdraw
/// Amount: UI units
pub async fn build_withdraw_tx(
    wallet: &str,
    market: &str,
    reserve: &str,
    amount: &str,
) -> Result<String> {
    let url = format!("{}/ktx/klend/withdraw", API_BASE);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "wallet": wallet,
        "market": market,
        "reserve": reserve,
        "amount": amount
    });
    let resp = client.post(&url).json(&body).send().await?;
    let data: Value = resp.json().await?;
    if let Some(tx) = data["transaction"].as_str() {
        Ok(tx.to_string())
    } else {
        anyhow::bail!(
            "Kamino API withdraw error: {}",
            data["message"].as_str().unwrap_or("unknown error")
        )
    }
}

/// Build a borrow transaction.
/// POST /ktx/klend/borrow
/// Amount: UI units
/// NOTE: Requires a prior deposit (obligation must already exist).
pub async fn build_borrow_tx(
    wallet: &str,
    market: &str,
    reserve: &str,
    amount: &str,
) -> Result<String> {
    let url = format!("{}/ktx/klend/borrow", API_BASE);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "wallet": wallet,
        "market": market,
        "reserve": reserve,
        "amount": amount
    });
    let resp = client.post(&url).json(&body).send().await?;
    let data: Value = resp.json().await?;
    if let Some(tx) = data["transaction"].as_str() {
        Ok(tx.to_string())
    } else {
        anyhow::bail!(
            "Kamino API borrow error: {}",
            data["message"].as_str().unwrap_or("unknown error")
        )
    }
}

/// Build a repay transaction.
/// POST /ktx/klend/repay
/// Amount: UI units
pub async fn build_repay_tx(
    wallet: &str,
    market: &str,
    reserve: &str,
    amount: &str,
) -> Result<String> {
    let url = format!("{}/ktx/klend/repay", API_BASE);
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "wallet": wallet,
        "market": market,
        "reserve": reserve,
        "amount": amount
    });
    let resp = client.post(&url).json(&body).send().await?;
    let data: Value = resp.json().await?;
    if let Some(tx) = data["transaction"].as_str() {
        Ok(tx.to_string())
    } else {
        anyhow::bail!(
            "Kamino API repay error: {}",
            data["message"].as_str().unwrap_or("unknown error")
        )
    }
}

/// Approximate current time as ISO 8601 string (no chrono dependency).
fn chrono_approx_now() -> String {
    // Use a fixed end time relative to compile; for runtime we use std::time
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    unix_to_iso(secs)
}

fn chrono_approx_yesterday() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    unix_to_iso(secs.saturating_sub(172800)) // 48h ago to be safe
}

fn unix_to_iso(secs: u64) -> String {
    // Minimal ISO 8601 formatter without chrono
    let s = secs;
    let days_since_epoch = s / 86400;
    let time_of_day = s % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let sec = time_of_day % 60;

    // Convert days since epoch to Y-M-D (Gregorian calendar)
    let (y, mo, d) = days_to_ymd(days_since_epoch);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z", y, mo, d, h, m, sec)
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}
