use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Urls;

/// Result of a region-accessibility check against the Polymarket geoblock endpoint.
#[derive(Debug, Clone)]
pub enum RegionStatus {
    /// Polymarket is accessible from this IP.
    Accessible,
    /// Polymarket is blocked for this IP — trading is not permitted.
    Restricted { country: String },
    /// The geoblock endpoint could not be reached or returned an unexpected response.
    /// Commands may still proceed with a warning; the caller decides the handling.
    Indeterminate { reason: String },
}

// ── Cache ──────────────────────────────────────────────────────────────────────

/// Cache TTL: Accessible result is trusted for 1 hour (region rarely changes).
const TTL_ACCESSIBLE_SECS: u64 = 3600;
/// Cache TTL: Restricted result expires sooner so VPN switches are picked up quickly.
const TTL_RESTRICTED_SECS: u64 = 900;
/// Indeterminate is never cached — always re-probe so transient network issues
/// don't permanently block the check.

#[derive(Serialize, Deserialize)]
struct RegionCache {
    status: String,
    #[serde(default)]
    country: Option<String>,
    checked_at: u64,
}

fn cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("polymarket")
        .join("region_cache.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn read_cache() -> Option<RegionStatus> {
    let path = cache_path();
    let raw = std::fs::read_to_string(&path).ok()?;
    let entry: RegionCache = serde_json::from_str(&raw).ok()?;
    let age = now_secs().saturating_sub(entry.checked_at);
    match entry.status.as_str() {
        "accessible" if age < TTL_ACCESSIBLE_SECS => Some(RegionStatus::Accessible),
        "restricted" if age < TTL_RESTRICTED_SECS => Some(RegionStatus::Restricted {
            country: entry.country.unwrap_or_else(|| "unknown".to_string()),
        }),
        _ => None,
    }
}

fn write_cache(status: &RegionStatus) {
    let entry = match status {
        RegionStatus::Accessible => RegionCache {
            status: "accessible".to_string(),
            country: None,
            checked_at: now_secs(),
        },
        RegionStatus::Restricted { country } => RegionCache {
            status: "restricted".to_string(),
            country: Some(country.clone()),
            checked_at: now_secs(),
        },
        RegionStatus::Indeterminate { .. } => return, // never cache
    };
    let path = cache_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(&entry) {
        let _ = std::fs::write(&path, json);
    }
}

// ── Geoblock probe ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GeoblockResponse {
    blocked: bool,
    #[serde(default)]
    country: Option<String>,
}

async fn probe_geoblock(client: &Client) -> RegionStatus {
    let url = format!("{}/api/geoblock", Urls::web());

    let resp = match client
        .get(&url)
        .timeout(std::time::Duration::from_secs(8))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return RegionStatus::Indeterminate {
                reason: format!("network error reaching geoblock endpoint: {e}"),
            };
        }
    };

    if !resp.status().is_success() {
        return RegionStatus::Indeterminate {
            reason: format!("geoblock endpoint returned HTTP {}", resp.status()),
        };
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();

    if !content_type.contains("application/json") {
        return RegionStatus::Indeterminate {
            reason: "geoblock endpoint returned non-JSON (possible Cloudflare challenge page)"
                .to_string(),
        };
    }

    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            return RegionStatus::Indeterminate {
                reason: format!("failed to read geoblock response body: {e}"),
            };
        }
    };

    match serde_json::from_str::<GeoblockResponse>(&body) {
        Ok(g) if g.blocked => RegionStatus::Restricted {
            country: g.country.unwrap_or_else(|| "unknown".to_string()),
        },
        Ok(_) => RegionStatus::Accessible,
        Err(e) => RegionStatus::Indeterminate {
            reason: format!("failed to parse geoblock response: {e}"),
        },
    }
}

/// Call `GET /api/geoblock` directly and update the local cache with the result.
/// Use this when the user explicitly requests a fresh check (`check-access`, `quickstart`).
pub async fn assess_readiness_fresh(client: &Client) -> RegionStatus {
    let result = probe_geoblock(client).await;
    write_cache(&result); // cache the fresh result so subsequent commands benefit
    result
}

/// Check region status with a local cache (TTL: Accessible=1h, Restricted=15min).
/// Cache hit on Accessible → zero network overhead for the typical case.
/// Indeterminate is never cached so transient errors don't permanently suppress checks.
pub async fn assess_readiness(client: &Client) -> RegionStatus {
    if let Some(cached) = read_cache() {
        return cached;
    }
    assess_readiness_fresh(client).await
}
