/// Polymarket authentication helpers.
///
/// L1: ClobAuth EIP-712 signed via `onchainos sign-message --type eip712` → derive API keys
/// L2: HMAC-SHA256 request signing with stored credentials
///
/// EIP712Domain MUST be included in the `types` field of the structured data JSON for
/// onchainos to compute the hash correctly (root cause from Hyperliquid investigation).
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;

use crate::config::{save_credentials, Credentials, Urls};
use crate::onchainos::sign_eip712;

// ─── L1: ClobAuth EIP-712 via onchainos ──────────────────────────────────────

/// Build the EIP-712 structured data JSON for a ClobAuth message.
/// Includes EIP712Domain in `types` — required by onchainos sign-message.
fn build_clob_auth_json(wallet_addr: &str, timestamp: u64, nonce: u64) -> String {
    serde_json::to_string(&serde_json::json!({
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"}
            ],
            "ClobAuth": [
                {"name": "address", "type": "address"},
                {"name": "timestamp", "type": "string"},
                {"name": "nonce", "type": "uint256"},
                {"name": "message", "type": "string"}
            ]
        },
        "primaryType": "ClobAuth",
        "domain": {
            "name": "ClobAuthDomain",
            "version": "1",
            "chainId": 137
        },
        "message": {
            "address": wallet_addr,
            "timestamp": timestamp.to_string(),
            "nonce": nonce,
            "message": "This message attests that I control the given wallet"
        }
    }))
    .expect("ClobAuth JSON serialization failed")
}

/// Sign a ClobAuth EIP-712 message via the onchainos wallet.
/// Returns (signature, timestamp, nonce).
async fn sign_clob_auth_onchainos(wallet_addr: &str, nonce: u64) -> Result<(String, u64, u64)> {
    let timestamp = chrono::Utc::now().timestamp() as u64;
    let json = build_clob_auth_json(wallet_addr, timestamp, nonce);
    let signature = sign_eip712(&json).await
        .context("ClobAuth EIP-712 signing via onchainos failed")?;
    Ok((signature, timestamp, nonce))
}

/// Build L1 HTTP headers from a ClobAuth signature.
pub fn l1_headers(address: &str, sig: &str, timestamp: u64, nonce: u64) -> Vec<(String, String)> {
    vec![
        ("POLY_ADDRESS".to_string(), address.to_string()),
        ("POLY_SIGNATURE".to_string(), sig.to_string()),
        ("POLY_TIMESTAMP".to_string(), timestamp.to_string()),
        ("POLY_NONCE".to_string(), nonce.to_string()),
    ]
}

// ─── L2: HMAC-SHA256 ─────────────────────────────────────────────────────────

type HmacSha256 = Hmac<Sha256>;

/// Compute HMAC-SHA256 signature for a CLOB API request.
/// message = timestamp + method.to_uppercase() + request_path + body
/// Returns base64url-encoded signature.
pub fn hmac_signature(
    secret_b64url: &str,
    timestamp: u64,
    method: &str,
    path: &str,
    body: &str,
) -> Result<String> {
    let padded = match secret_b64url.len() % 4 {
        2 => format!("{}==", secret_b64url),
        3 => format!("{}=", secret_b64url),
        _ => secret_b64url.to_string(),
    };
    let secret_bytes = general_purpose::URL_SAFE
        .decode(&padded)
        .with_context(|| format!("decoding base64url secret (len={})", secret_b64url.len()))?;

    let message = format!("{}{}{}{}", timestamp, method.to_uppercase(), path, body);

    let mut mac = HmacSha256::new_from_slice(&secret_bytes).context("creating HMAC")?;
    mac.update(message.as_bytes());
    let result = mac.finalize().into_bytes();
    Ok(general_purpose::URL_SAFE.encode(result))
}

/// Build L2 HTTP headers for an authenticated CLOB request.
pub fn l2_headers(
    address: &str,
    api_key: &str,
    secret: &str,
    passphrase: &str,
    method: &str,
    path: &str,
    body: &str,
) -> Result<Vec<(String, String)>> {
    let timestamp = chrono::Utc::now().timestamp() as u64;
    let sig = hmac_signature(secret, timestamp, method, path, body)?;
    Ok(vec![
        ("POLY_ADDRESS".to_string(), address.to_string()),
        ("POLY_SIGNATURE".to_string(), sig),
        ("POLY_TIMESTAMP".to_string(), timestamp.to_string()),
        ("POLY_API_KEY".to_string(), api_key.to_string()),
        ("POLY_PASSPHRASE".to_string(), passphrase.to_string()),
    ])
}

// ─── API key management ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ApiKeyResponse {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
    /// Proxy wallet returned by Polymarket when the account has been set up via polymarket.com.
    #[serde(rename = "proxyWallet", default)]
    pub proxy_wallet: Option<String>,
}

/// Create new API keys using L1 auth with onchainos wallet.
pub async fn create_api_key(client: &Client, wallet_addr: &str, nonce: u64) -> Result<Credentials> {
    let (sig, timestamp, nonce_used) = sign_clob_auth_onchainos(wallet_addr, nonce).await?;
    let headers = l1_headers(wallet_addr, &sig, timestamp, nonce_used);

    let mut req = client.post(format!("{}/auth/api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

    if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!("Polymarket /auth/api-key failed: {}\nResponse: {}", err, resp);
    }

    let api_key_resp: ApiKeyResponse = serde_json::from_value(resp.clone())
        .with_context(|| format!("parsing api-key response: {}", resp))?;

    let creds = Credentials {
        api_key: api_key_resp.api_key,
        secret: api_key_resp.secret,
        passphrase: api_key_resp.passphrase,
        nonce,
        signing_address: wallet_addr.to_string(),
        proxy_wallet: api_key_resp.proxy_wallet,
    };
    save_credentials(&creds)?;
    Ok(creds)
}

/// Derive existing API keys using L1 auth + same nonce.
pub async fn derive_api_key(client: &Client, wallet_addr: &str, nonce: u64) -> Result<Credentials> {
    let (sig, timestamp, _) = sign_clob_auth_onchainos(wallet_addr, nonce).await?;
    let headers = l1_headers(wallet_addr, &sig, timestamp, nonce);

    let mut req = client.get(format!("{}/auth/derive-api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

    if resp.get("error").is_some() {
        anyhow::bail!("derive-api-key rejected: {}", resp);
    }

    let api_key_resp: ApiKeyResponse = serde_json::from_value(resp.clone())
        .with_context(|| format!("parsing derive-api-key response: {}", resp))?;

    let creds = Credentials {
        api_key: api_key_resp.api_key,
        secret: api_key_resp.secret,
        passphrase: api_key_resp.passphrase,
        nonce,
        signing_address: wallet_addr.to_string(),
        proxy_wallet: api_key_resp.proxy_wallet,
    };
    save_credentials(&creds)?;
    Ok(creds)
}

/// Load stored credentials or auto-derive them using the onchainos wallet.
/// Re-derives if the cached credentials were for a different wallet address.
pub async fn ensure_credentials(client: &Client, wallet_addr: &str) -> Result<Credentials> {
    // Check environment variables first
    let env_key = std::env::var("POLYMARKET_API_KEY").unwrap_or_default();
    let env_secret = std::env::var("POLYMARKET_SECRET").unwrap_or_default();
    let env_pass = std::env::var("POLYMARKET_PASSPHRASE").unwrap_or_default();

    if !env_key.is_empty() && !env_secret.is_empty() && !env_pass.is_empty() {
        return Ok(Credentials {
            api_key: env_key,
            secret: env_secret,
            passphrase: env_pass,
            nonce: 0,
            signing_address: wallet_addr.to_string(),
            proxy_wallet: None,
        });
    }

    // Try loading from file — only use if it matches the current wallet
    if let Some(creds) = crate::config::load_credentials()? {
        if creds.signing_address.to_lowercase() == wallet_addr.to_lowercase() {
            return Ok(creds);
        }
        eprintln!("[polymarket] Wallet address changed, re-deriving API credentials...");
    }

    // Auto-derive via onchainos wallet EIP-712 signing
    eprintln!("[polymarket] Deriving API credentials for wallet {}...", wallet_addr);
    match derive_api_key(client, wallet_addr, 0).await {
        Ok(c) => Ok(c),
        Err(_) => create_api_key(client, wallet_addr, 0).await,
    }
}
