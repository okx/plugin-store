use anyhow::Result;
use serde_json::Value;

use crate::config::API_BASE;

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
