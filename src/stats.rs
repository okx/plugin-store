use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Map of plugin name → download count.
pub type StatsMap = HashMap<String, u64>;

fn parse_stats(raw: HashMap<String, serde_json::Value>) -> StatsMap {
    raw.into_iter()
        .filter_map(|(k, v)| {
            let n = match &v {
                serde_json::Value::Number(n) => n.as_u64(),
                serde_json::Value::String(s) => s.parse().ok(),
                _ => None,
            };
            n.map(|n| (k, n))
        })
        .collect()
}

#[derive(Debug, Serialize, Deserialize)]
struct ReportPayload {
    name: String,
    version: String,
}

/// Resolve stats base URL: registry value takes priority, fallback to env var.
fn resolve_url(registry_url: Option<&str>) -> Option<String> {
    registry_url
        .map(|s| s.to_string())
        .or_else(|| std::env::var("PLUGIN_STORE_STATS_URL").ok())
}

/// Fetch download counts from the stats API.
/// GET {stats_url}/counts → {"plugin-name": 123, ...}
/// Returns an empty map on any error or if the URL is not configured.
pub async fn fetch(registry_url: Option<&str>) -> StatsMap {
    let Some(base) = resolve_url(registry_url) else {
        return HashMap::new();
    };
    let url = format!("{}/counts", base.trim_end_matches('/'));
    let Ok(resp) = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "plugin-store")
        .send()
        .await
    else {
        return HashMap::new();
    };
    let raw: HashMap<String, serde_json::Value> = resp.json().await.unwrap_or_default();
    parse_stats(raw)
}

/// Report a successful install to the stats API (fire-and-forget).
/// POST {stats_url}/install → {"name": "...", "version": "..."}
/// Silently does nothing if no stats URL is configured.
pub async fn report_install(name: &str, version: &str, registry_url: Option<&str>) {
    let Some(base) = resolve_url(registry_url) else {
        return;
    };
    let url = format!("{}/install", base.trim_end_matches('/'));
    let payload = ReportPayload {
        name: name.to_string(),
        version: version.to_string(),
    };
    let _ = reqwest::Client::new()
        .post(&url)
        .header("User-Agent", "plugin-store")
        .json(&payload)
        .send()
        .await;
}
