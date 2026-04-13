/// Series market resolution for recurring short-duration markets.
///
/// Polymarket runs recurring "Up or Down" series markets on crypto assets:
///
/// 5-minute slots  — NYSE trading hours only (9:30 AM – 4:00 PM ET, Mon–Fri)
///   Slug: `{asset}-updown-5m-{unix_start_utc}`
///   IDs:  btc-5m, eth-5m, sol-5m, xrp-5m
///
/// 15-minute slots — NYSE trading hours only
///   Slug: `{asset}-updown-15m-{unix_start_utc}`
///   IDs:  btc-15m, eth-15m, sol-15m, xrp-15m
///
/// 4-hour slots    — runs 24/7 (6 slots per day, every 4 hours)
///   Slug: `{asset}-updown-4h-{unix_start_utc}`
///   IDs:  btc-4h, eth-4h, sol-4h, xrp-4h
///
/// Aliases accepted: "btc"/"bitcoin", "eth"/"ethereum", "sol"/"solana", "xrp"
/// plus interval suffixes: "btc-5m", "btc-15m", "btc-4h", etc.
use anyhow::{bail, Result};
use reqwest::Client;

use crate::api::GammaMarket;

// ─── Series registry ──────────────────────────────────────────────────────────

pub struct SeriesSpec {
    pub id: &'static str,              // canonical ID, e.g. "btc-5m"
    pub asset: &'static str,           // slug prefix, e.g. "btc"
    pub display: &'static str,         // human name, e.g. "Bitcoin"
    pub interval_secs: u64,            // window length in seconds
    pub interval_label: &'static str,  // e.g. "5m", "15m", "4h"
    pub nyse_hours_only: bool,         // if true, only active during NYSE trading hours
}

pub const SERIES: &[SeriesSpec] = &[
    // 5-minute slots (NYSE hours)
    SeriesSpec { id: "btc-5m",  asset: "btc", display: "Bitcoin",  interval_secs: 300,   interval_label: "5m",  nyse_hours_only: true  },
    SeriesSpec { id: "eth-5m",  asset: "eth", display: "Ethereum", interval_secs: 300,   interval_label: "5m",  nyse_hours_only: true  },
    SeriesSpec { id: "sol-5m",  asset: "sol", display: "Solana",   interval_secs: 300,   interval_label: "5m",  nyse_hours_only: true  },
    SeriesSpec { id: "xrp-5m",  asset: "xrp", display: "XRP",     interval_secs: 300,   interval_label: "5m",  nyse_hours_only: true  },
    // 15-minute slots (NYSE hours)
    SeriesSpec { id: "btc-15m", asset: "btc", display: "Bitcoin",  interval_secs: 900,   interval_label: "15m", nyse_hours_only: true  },
    SeriesSpec { id: "eth-15m", asset: "eth", display: "Ethereum", interval_secs: 900,   interval_label: "15m", nyse_hours_only: true  },
    SeriesSpec { id: "sol-15m", asset: "sol", display: "Solana",   interval_secs: 900,   interval_label: "15m", nyse_hours_only: true  },
    SeriesSpec { id: "xrp-15m", asset: "xrp", display: "XRP",     interval_secs: 900,   interval_label: "15m", nyse_hours_only: true  },
    // 4-hour slots (24/7 — no NYSE hours restriction)
    SeriesSpec { id: "btc-4h",  asset: "btc", display: "Bitcoin",  interval_secs: 14400, interval_label: "4h",  nyse_hours_only: false },
    SeriesSpec { id: "eth-4h",  asset: "eth", display: "Ethereum", interval_secs: 14400, interval_label: "4h",  nyse_hours_only: false },
    SeriesSpec { id: "sol-4h",  asset: "sol", display: "Solana",   interval_secs: 14400, interval_label: "4h",  nyse_hours_only: false },
    SeriesSpec { id: "xrp-4h",  asset: "xrp", display: "XRP",     interval_secs: 14400, interval_label: "4h",  nyse_hours_only: false },
];

/// Parse a series string into a SeriesSpec.
/// Accepts full IDs ("btc-5m", "eth-15m", "btc-4h"), bare asset names ("btc",
/// "bitcoin"), and asset+interval combos ("btc-updown-5m", "eth-updown-15m").
/// Bare asset names ("btc", "bitcoin") resolve to the 5-minute series.
pub fn parse_series(s: &str) -> Option<&'static SeriesSpec> {
    let lower = s.to_lowercase();
    SERIES.iter().find(|spec| {
        lower == spec.id
            || lower == format!("{}-updown-{}", spec.asset, spec.interval_label)
            // bare asset name → default to 5m
            || (spec.interval_label == "5m" && (lower == spec.asset || lower == spec.display.to_lowercase()))
    })
}

/// Returns true if the string looks like a series identifier.
pub fn is_series_id(s: &str) -> bool {
    parse_series(s).is_some()
}

// ─── NYSE trading hours ───────────────────────────────────────────────────────

/// Return the ET (Eastern Time) UTC offset in seconds for a given Unix timestamp.
/// Accounts for US DST: EDT (UTC-4) from 2nd Sunday of March to 1st Sunday of November,
/// EST (UTC-5) otherwise.
fn et_offset_secs(unix_ts: u64) -> i64 {
    use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};

    let dt = DateTime::from_timestamp(unix_ts as i64, 0).unwrap_or_else(|| Utc::now());
    let year = dt.year();

    // 2nd Sunday of March → DST starts at 2 AM EST = 7 AM UTC
    let dst_start_day = nth_weekday_of_month(year, 3, Weekday::Sun, 2);
    let dst_start = NaiveDate::from_ymd_opt(year, 3, dst_start_day)
        .and_then(|d| d.and_hms_opt(7, 0, 0))
        .map(|dt| dt.and_utc())
        .unwrap_or(dt);

    // 1st Sunday of November → DST ends at 2 AM EDT = 6 AM UTC
    let dst_end_day = nth_weekday_of_month(year, 11, Weekday::Sun, 1);
    let dst_end = NaiveDate::from_ymd_opt(year, 11, dst_end_day)
        .and_then(|d| d.and_hms_opt(6, 0, 0))
        .map(|dt| dt.and_utc())
        .unwrap_or(dt);

    if dt >= dst_start && dt < dst_end {
        -4 * 3600 // EDT
    } else {
        -5 * 3600 // EST
    }
}

/// Find the nth occurrence of a weekday in a given year/month.
fn nth_weekday_of_month(year: i32, month: u32, weekday: chrono::Weekday, n: u32) -> u32 {
    use chrono::{Datelike, NaiveDate};
    let mut count = 0u32;
    for day in 1u32..=31 {
        if let Some(d) = NaiveDate::from_ymd_opt(year, month, day) {
            if d.weekday() == weekday {
                count += 1;
                if count == n {
                    return day;
                }
            }
        } else {
            break;
        }
    }
    1 // fallback
}

/// Check whether a Unix timestamp falls within NYSE trading hours:
/// 9:30 AM – 4:00 PM ET, Monday–Friday.
pub fn is_in_trading_hours(unix_ts: u64) -> bool {
    use chrono::{DateTime, Datelike, Timelike, Weekday};

    // Shift timestamp to ET by adding the offset (negative), giving a "fake UTC"
    // whose hour/minute/weekday fields read as ET local time.
    let et_ts = unix_ts as i64 + et_offset_secs(unix_ts);
    let dt = match DateTime::from_timestamp(et_ts, 0) {
        Some(d) => d,
        None => return false,
    };

    // Weekend check
    if matches!(dt.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }

    // 9:30 AM (570 min) inclusive to 4:00 PM (960 min) exclusive
    let mins = dt.hour() * 60 + dt.minute();
    mins >= 570 && mins < 960
}

/// Compute how many seconds remain in the current trading session,
/// or 0 if currently outside trading hours.
pub fn seconds_remaining_in_session(unix_ts: u64) -> u64 {
    if !is_in_trading_hours(unix_ts) {
        return 0;
    }
    let et_ts = unix_ts as i64 + et_offset_secs(unix_ts);
    let dt = match chrono::DateTime::from_timestamp(et_ts, 0) {
        Some(d) => d,
        None => return 0,
    };
    use chrono::Timelike;
    let end_of_session_mins = 16 * 60u32; // 4:00 PM
    let current_mins = dt.hour() * 60 + dt.minute();
    let remaining_mins = end_of_session_mins.saturating_sub(current_mins);
    (remaining_mins as u64) * 60 - dt.second() as u64
}

/// Compute seconds until the next NYSE trading session opens (0 if currently open).
pub fn seconds_until_trading_opens(from_unix: u64) -> u64 {
    if is_in_trading_hours(from_unix) {
        return 0;
    }
    // Walk forward in 60-second steps; cap at 7 days
    let mut t = from_unix;
    for _ in 0..(7 * 24 * 60) {
        t += 60;
        if is_in_trading_hours(t) {
            return t - from_unix;
        }
    }
    0
}

// ─── Slot resolution ──────────────────────────────────────────────────────────

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Build the Gamma slug for a series slot.
fn slot_slug(spec: &SeriesSpec, slot_start: u64) -> String {
    format!("{}-updown-{}-{}", spec.asset, spec.interval_label, slot_start)
}

/// Round a Unix timestamp down to the nearest slot boundary.
fn floor_to_slot(unix_ts: u64, interval_secs: u64) -> u64 {
    (unix_ts / interval_secs) * interval_secs
}

/// Fetch the current accepting market for a series.
///
/// Tries the current slot and the next slot (handles the brief gap between
/// when one slot closes and the next one opens). Returns the first one
/// that is `accepting_orders: true`.
///
/// For NYSE-hours-only series (5m, 15m), fails clearly outside trading hours.
/// For 24/7 series (4h), always attempts the lookup.
pub async fn get_current_slot(client: &Client, spec: &SeriesSpec) -> Result<GammaMarket> {
    let now = now_unix();

    let current = floor_to_slot(now, spec.interval_secs);

    // Try current slot, then next (slot may have just closed, next may be open)
    for ts in [current, current + spec.interval_secs] {
        let slug = slot_slug(spec, ts);
        match crate::api::get_gamma_market_by_slug(client, &slug).await {
            Ok(m) if m.accepting_orders => return Ok(m),
            Ok(_) => {}  // market exists but not accepting orders
            Err(_) => {} // not yet created
        }
    }

    bail!(
        "No open {} {} market found for the current slot (around {}). \
         The window may be transitioning — wait a few seconds and retry.",
        spec.display,
        spec.id,
        chrono::DateTime::from_timestamp(current as i64, 0)
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| current.to_string())
    );
}

/// Resolve a series ID to the slug of the current accepting market slot.
/// Used by buy/sell to transparently handle series identifiers as market_ids.
pub async fn resolve_to_slug(client: &Client, series_id: &str) -> Result<String> {
    let spec = parse_series(series_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown series '{}'. Supported: btc-5m/15m/4h, eth-5m/15m/4h, sol-5m/15m/4h, xrp-5m/15m/4h", series_id))?;
    let market = get_current_slot(client, spec).await?;
    market.slug.ok_or_else(|| anyhow::anyhow!("Series market has no slug"))
}

/// Resolve a series ID to the current accepting GammaMarket (avoids double Gamma fetch in buy/sell).
pub async fn resolve_to_market(client: &Client, series_id: &str) -> Result<crate::api::GammaMarket> {
    let spec = parse_series(series_id)
        .ok_or_else(|| anyhow::anyhow!("Unknown series '{}'. Supported: btc-5m, eth-5m, sol-5m, xrp-5m", series_id))?;
    get_current_slot(client, spec).await
}

// ─── get-series output helpers ────────────────────────────────────────────────

pub struct SlotSummary {
    pub slug: String,
    pub start_unix: u64,
    pub end_unix: u64,
    pub market: Option<GammaMarket>,
}

/// Fetch info for the current and next slot of a series.
pub async fn get_series_info(
    client: &Client,
    spec: &SeriesSpec,
) -> Result<(bool, SlotSummary, SlotSummary)> {
    let now = now_unix();
    // For NYSE-restricted series, report whether we're in trading hours.
    // For 24/7 series (4h), always treat as "in hours".
    let in_hours = !spec.nyse_hours_only || is_in_trading_hours(now);

    let current_start = floor_to_slot(now, spec.interval_secs);
    let next_start = current_start + spec.interval_secs;

    let current_slug = slot_slug(spec, current_start);
    let next_slug = slot_slug(spec, next_start);

    // Fetch both in parallel
    let (current_market, next_market) = tokio::join!(
        crate::api::get_gamma_market_by_slug(client, &current_slug),
        crate::api::get_gamma_market_by_slug(client, &next_slug),
    );

    let current = SlotSummary {
        slug: current_slug,
        start_unix: current_start,
        end_unix: current_start + spec.interval_secs,
        market: current_market.ok(),
    };
    let next = SlotSummary {
        slug: next_slug,
        start_unix: next_start,
        end_unix: next_start + spec.interval_secs,
        market: next_market.ok(),
    };

    Ok((in_hours, current, next))
}
