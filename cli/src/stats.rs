use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OkxReportPayload {
    plugin_name: String,
    div_id: String,
}

/// Recover the HMAC secret at runtime via XOR deobfuscation.
/// The key is stored as XOR-encoded bytes — not visible as a plain string
/// in the binary, hex dump, or `strings` output.
fn hmac_secret() -> Vec<u8> {
    // XOR mask (arbitrary, compiled into binary)
    const MASK: &[u8] = &[
        0xA3, 0x7F, 0x1B, 0xE2, 0x54, 0xC8, 0x90, 0x3D,
        0x6E, 0xF1, 0x82, 0x47, 0xD5, 0x09, 0xBB, 0x6C,
        0x2A, 0x95, 0xE7, 0x13, 0x78, 0x4D, 0xA6, 0xF0,
        0x31, 0xCC, 0x5E, 0x8A, 0x19, 0xD3, 0x67, 0xB4,
        0x0F, 0xE5, 0x42, 0x9D, 0x7A, 0x26, 0xC1, 0x58,
        0x3B, 0xAF, 0x64,
    ];
    // Encoded = secret XOR mask (pre-computed)
    const ENCODED: &[u8] = &[
        0x9B, 0x30, 0x7C, 0xD7, 0x35, 0x99, 0xC0, 0x6A,
        0x31, 0xB8, 0xD0, 0x23, 0xAF, 0x42, 0xD0, 0x5C,
        0x66, 0xDB, 0xB0, 0x77, 0x35, 0x34, 0x94, 0xC3,
        0x66, 0xAE, 0x3C, 0xE7, 0x63, 0xE4, 0x02, 0xD7,
        0x5C, 0x89, 0x0E, 0xD4, 0x2A, 0x17, 0x89, 0x02,
        0x4E, 0xC7, 0x03,
    ];
    MASK.iter().zip(ENCODED.iter()).map(|(m, e)| m ^ e).collect()
}

/// Generate a stable device ID from machine fingerprint + HMAC signature.
/// Format: 32-char device hash + 8-char HMAC sig = 40 chars total.
fn generate_device_token() -> String {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let raw = format!("{}:{}:{}:{}", hostname, os, arch, home);

    // Device ID: SHA-256 of fingerprint, first 32 hex chars
    let device_hash = hex::encode(Sha256::digest(raw.as_bytes()));
    let device_id = &device_hash[..32];

    // HMAC signature: SHA-256(secret + device_id), first 8 hex chars
    let secret = hmac_secret();
    let mut hmac_hasher = Sha256::new();
    hmac_hasher.update(&secret);
    hmac_hasher.update(device_id.as_bytes());
    let sig = hex::encode(hmac_hasher.finalize());
    let sig_short = &sig[..8];

    format!("{}{}", device_id, sig_short)
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

/// Report a successful install (fire-and-forget, dual endpoint).
/// 1. POST {stats_url}/install → Vercel stats (existing)
/// 2. POST okx API → OKX download report (new)
pub async fn report_install(name: &str, version: &str, registry_url: Option<&str>) {
    let client = reqwest::Client::new();
    let device_token = generate_device_token();

    // ── Vercel stats (existing) ──
    if let Some(base) = resolve_url(registry_url) {
        let url = format!("{}/install", base.trim_end_matches('/'));
        let payload = ReportPayload {
            name: name.to_string(),
            version: version.to_string(),
        };
        let _ = client
            .post(&url)
            .header("User-Agent", "plugin-store")
            .json(&payload)
            .send()
            .await;
    }

    // ── OKX download report (new) ──
    let okx_payload = OkxReportPayload {
        plugin_name: name.to_string(),
        div_id: device_token,
    };
    let _ = client
        .post("https://www.okx.com/priapi/v1/wallet/plugins/download/report")
        .header("User-Agent", "plugin-store")
        .json(&okx_payload)
        .send()
        .await;
}
