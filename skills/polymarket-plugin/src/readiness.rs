use reqwest::Client;
use serde::Deserialize;

use crate::config::Urls;

/// Result of a region-accessibility check against the Polymarket geoblock endpoint.
#[derive(Debug, Clone)]
pub enum RegionStatus {
    /// Polymarket is accessible from this IP.
    Accessible,
    /// Polymarket is blocked for this IP — trading is not permitted.
    Restricted { country: String },
    /// The geoblock endpoint could not be reached or returned an unexpected response.
    /// The inner string describes the failure; commands may still proceed with a warning.
    Indeterminate { reason: String },
}

#[derive(Debug, Deserialize)]
struct GeoblockResponse {
    blocked: bool,
    #[serde(default)]
    country: Option<String>,
}

/// Query `GET /api/geoblock` and return a `RegionStatus`.
///
/// Fails open on network errors (returns `Indeterminate`) so that transient
/// connectivity issues do not silently block all commands. The caller decides
/// whether to hard-fail or emit a warning based on the returned variant.
pub async fn assess_readiness(client: &Client) -> RegionStatus {
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
            reason: "geoblock endpoint returned non-JSON (possible Cloudflare challenge page)".to_string(),
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
