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

/// Build L1 HTTP headers for ERC-1271 (deposit wallet) auth.
/// Includes POLY_SIGNATURE_TYPE: 3 so the CLOB verifies via isValidSignature
/// rather than ECDSA recovery against POLY_ADDRESS directly.
pub fn l1_headers_erc1271(address: &str, sig: &str, timestamp: u64, nonce: u64) -> Vec<(String, String)> {
    vec![
        ("POLY_ADDRESS".to_string(), address.to_string()),
        ("POLY_SIGNATURE".to_string(), sig.to_string()),
        ("POLY_TIMESTAMP".to_string(), timestamp.to_string()),
        ("POLY_NONCE".to_string(), nonce.to_string()),
        ("POLY_SIGNATURE_TYPE".to_string(), "3".to_string()),
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

// ─── Builder credentials ──────────────────────────────────────────────────────

/// Credentials derived from `POST /auth/builder-api-key`.
/// Used for authenticating relayer WALLET-CREATE and WALLET batch submissions.
///
/// Note: the builder endpoint returns `key` (not `apiKey` like the CLOB endpoint).
#[derive(Debug, Clone, Deserialize)]
pub struct BuilderCredentials {
    #[serde(rename = "key")]
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
}

/// Build POLY_BUILDER_* headers for a relayer request.
///
/// Builder HMAC uses the same URL-safe base64 decoded secret and HMAC-SHA256
/// algorithm as CLOB L2 auth — only the header names differ.
pub fn builder_l2_headers(
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
        ("POLY_BUILDER_API_KEY".to_string(), api_key.to_string()),
        ("POLY_BUILDER_PASSPHRASE".to_string(), passphrase.to_string()),
        ("POLY_BUILDER_SIGNATURE".to_string(), sig),
        ("POLY_BUILDER_TIMESTAMP".to_string(), timestamp.to_string()),
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
        deposit_wallet: None,
        mode: crate::config::TradingMode::default(),
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
        deposit_wallet: None,
        mode: crate::config::TradingMode::default(),
    };
    save_credentials(&creds)?;
    Ok(creds)
}

/// Create CLOB API keys for a deposit wallet using ERC-1271 (POLY_SIGNATURE_TYPE=3).
///
/// The active onchainos key (EOA) signs the ClobAuth EIP-712 message, but POLY_ADDRESS
/// is set to the deposit wallet. The CLOB verifies via ERC-1271 by calling
/// `deposit_wallet.isValidSignature(clob_auth_hash, ecdsa_sig)`.
/// Returns credentials with signing_address = deposit_wallet.
pub async fn create_api_key_erc1271(client: &Client, deposit_wallet: &str, nonce: u64) -> Result<Credentials> {
    let (sig, timestamp, nonce_used) = sign_clob_auth_onchainos(deposit_wallet, nonce).await?;
    let headers = l1_headers_erc1271(deposit_wallet, &sig, timestamp, nonce_used);

    let mut req = client.post(format!("{}/auth/api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

    if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!("Polymarket /auth/api-key (ERC-1271) failed: {}\nResponse: {}", err, resp);
    }

    let api_key_resp: ApiKeyResponse = serde_json::from_value(resp.clone())
        .with_context(|| format!("parsing api-key (ERC-1271) response: {}", resp))?;

    let creds = Credentials {
        api_key: api_key_resp.api_key,
        secret: api_key_resp.secret,
        passphrase: api_key_resp.passphrase,
        nonce,
        signing_address: deposit_wallet.to_string(),
        proxy_wallet: None,
        deposit_wallet: None,
        mode: crate::config::TradingMode::DepositWallet,
    };
    save_credentials(&creds)?;
    Ok(creds)
}

/// Derive existing CLOB API keys for a deposit wallet using ERC-1271.
pub async fn derive_api_key_erc1271(client: &Client, deposit_wallet: &str, nonce: u64) -> Result<Credentials> {
    let (sig, timestamp, _) = sign_clob_auth_onchainos(deposit_wallet, nonce).await?;
    let headers = l1_headers_erc1271(deposit_wallet, &sig, timestamp, nonce);

    let mut req = client.get(format!("{}/auth/derive-api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

    if resp.get("error").is_some() {
        anyhow::bail!("derive-api-key (ERC-1271) rejected: {}", resp);
    }

    let api_key_resp: ApiKeyResponse = serde_json::from_value(resp.clone())
        .with_context(|| format!("parsing derive-api-key (ERC-1271) response: {}", resp))?;

    let creds = Credentials {
        api_key: api_key_resp.api_key,
        secret: api_key_resp.secret,
        passphrase: api_key_resp.passphrase,
        nonce,
        signing_address: deposit_wallet.to_string(),
        proxy_wallet: None,
        deposit_wallet: None,
        mode: crate::config::TradingMode::DepositWallet,
    };
    save_credentials(&creds)?;
    Ok(creds)
}

/// Load or derive CLOB credentials for a deposit wallet (ERC-1271 / POLY_1271 mode).
///
/// The deposit wallet needs its own CLOB API key — separate from the EOA's key —
/// because the CLOB validates order.signer == address_of(API_KEY), and for POLY_1271
/// orders signer = maker = deposit_wallet_address.
///
/// The CLOB verifies the auth signature via ERC-1271 (POLY_SIGNATURE_TYPE: 3), so
/// the active onchainos key (EOA) signs on behalf of the deposit wallet.
pub async fn ensure_credentials_deposit_wallet(client: &Client, deposit_wallet: &str) -> Result<Credentials> {
    if let Some(creds) = crate::config::load_credentials_for(deposit_wallet)? {
        return Ok(creds);
    }

    eprintln!("[polymarket] Deriving ERC-1271 CLOB credentials for deposit wallet {}...", deposit_wallet);

    match derive_api_key_erc1271(client, deposit_wallet, 0).await {
        Ok(c) => Ok(c),
        Err(_) => create_api_key_erc1271(client, deposit_wallet, 0).await,
    }
}

/// Readonly API key — returned by `/auth/readonly-api-key` (CLOB v2).
/// Carries the same api_key/secret/passphrase triplet but with write operations rejected
/// by the CLOB server. Useful for monitoring scripts, dashboards, and CI pipelines.
#[derive(Debug, Deserialize)]
pub struct ReadonlyApiKeyResponse {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
}

/// Create a read-only API key via L1 auth (CLOB v2 endpoint).
///
/// The returned key can be used for `GET /orders`, `GET /trades`, balance lookups, etc.
/// but the CLOB server will reject any write operations (order placement, cancellation).
/// Not persisted to `creds.json` — caller is responsible for storing/displaying it.
pub async fn create_readonly_api_key(
    client: &Client,
    wallet_addr: &str,
) -> Result<ReadonlyApiKeyResponse> {
    let nonce: u64 = 0;
    let (sig, timestamp, nonce_used) = sign_clob_auth_onchainos(wallet_addr, nonce).await?;
    let headers = l1_headers(wallet_addr, &sig, timestamp, nonce_used);

    let mut req = client.post(format!("{}/auth/readonly-api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

    if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!("Polymarket /auth/readonly-api-key failed: {}\nResponse: {}", err, resp);
    }

    serde_json::from_value(resp.clone())
        .with_context(|| format!("parsing readonly-api-key response: {}", resp))
}

/// Load stored credentials or auto-derive them using the onchainos wallet.
/// Re-derives if the cached credentials were for a different wallet address.
///
/// On first use (no creds stored), auto-detects trading mode:
///   - If Polymarket returns a proxy wallet address → mode = PolyProxy
///   - Otherwise → mode = EOA, with a hint to run `polymarket setup-proxy`
pub async fn ensure_credentials(client: &Client, wallet_addr: &str) -> Result<Credentials> {
    use crate::config::TradingMode;

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
            deposit_wallet: None,
            mode: TradingMode::Eoa,
        });
    }

    // Try loading from the multi-wallet store for this specific address.
    // No address check needed — load_credentials_for only returns an entry
    // when the address matches exactly.
    if let Some(creds) = crate::config::load_credentials_for(wallet_addr)? {
        return Ok(creds);
    }

    // Auto-derive via onchainos wallet EIP-712 signing
    eprintln!("[polymarket] Deriving API credentials for wallet {}...", wallet_addr);
    let mut creds = match derive_api_key(client, wallet_addr, 0).await {
        Ok(c) => c,
        Err(_) => create_api_key(client, wallet_addr, 0).await?,
    };

    // Load any previously-saved mode so we can respect explicit user choices
    // (e.g. setup-deposit-wallet → DepositWallet; switch-mode → user-chosen).
    // Without this, the auto-pick below clobbers the saved mode every time
    // ensure_credentials runs, making setup-deposit-wallet's mode write useless
    // for any wallet that also has a proxy.
    let saved_mode = crate::config::load_credentials_for(wallet_addr)
        .ok()
        .flatten()
        .map(|c| c.mode);

    // Auto-detect trading mode based on whether a proxy or deposit wallet exists.
    // Priority: API key response proxyWallet field → /profile API → on-chain factory check.
    // The /profile API is unreliable in CLOB V2 and may return empty — always fall back to on-chain.
    if creds.proxy_wallet.is_none() {
        // Try /profile API first (fast, no RPC needed)
        if let Ok(Some(proxy)) = crate::api::get_proxy_wallet(client, wallet_addr).await {
            creds.proxy_wallet = Some(proxy);
        }
        // Fallback: check on-chain via proxy factory eth_call (reliable even when /profile is down)
        if creds.proxy_wallet.is_none() {
            if let Some((proxy_addr, exists)) = crate::onchainos::get_existing_proxy(wallet_addr).await.ok().flatten() {
                if exists {
                    creds.proxy_wallet = Some(proxy_addr);
                }
            }
        }
    }

    // Check for deposit wallet on-chain. Detect independently of proxy existence —
    // a wallet can hold both (proxy from V1 era + deposit wallet from V2 setup).
    if creds.deposit_wallet.is_none() {
        if let Some(dw) = crate::onchainos::get_existing_deposit_wallet(wallet_addr).await {
            creds.deposit_wallet = Some(dw);
        }
    }

    // Mode selection: respect user's saved explicit choice when its prerequisites
    // still hold (DepositWallet needs deposit_wallet present; PolyProxy needs proxy).
    // Fall through to auto-pick only when no compatible saved mode exists.
    creds.mode = match saved_mode {
        Some(TradingMode::DepositWallet) if creds.deposit_wallet.is_some() => {
            eprintln!(
                "[polymarket] Using saved DEPOSIT_WALLET mode (deposit wallet: {}).",
                creds.deposit_wallet.as_deref().unwrap_or("")
            );
            TradingMode::DepositWallet
        }
        Some(TradingMode::PolyProxy) if creds.proxy_wallet.is_some() => {
            eprintln!(
                "[polymarket] Using saved POLY_PROXY mode (proxy: {}).",
                creds.proxy_wallet.as_deref().unwrap_or("")
            );
            TradingMode::PolyProxy
        }
        Some(TradingMode::Eoa) => {
            eprintln!("[polymarket] Using saved EOA mode.");
            TradingMode::Eoa
        }
        _ => {
            // No compatible saved mode — auto-pick. Prefer DepositWallet over
            // PolyProxy when both available, since deposit wallet is gasless and
            // is the v0.6.0+ default for new users.
            if creds.deposit_wallet.is_some() {
                eprintln!(
                    "[polymarket] Deposit wallet detected: {}. Using DEPOSIT_WALLET mode (gasless, POLY_1271).",
                    creds.deposit_wallet.as_deref().unwrap_or("")
                );
                TradingMode::DepositWallet
            } else if creds.proxy_wallet.is_some() {
                eprintln!(
                    "[polymarket] Proxy wallet detected: {}. Using POLY_PROXY mode (no POL needed for trading).",
                    creds.proxy_wallet.as_deref().unwrap_or("")
                );
                TradingMode::PolyProxy
            } else {
                eprintln!(
                    "[polymarket] No proxy or deposit wallet found. Using EOA mode (requires POL for gas).\n\
                     Tip: run `polymarket setup-proxy` to create a proxy wallet and switch to gasless trading."
                );
                TradingMode::Eoa
            }
        }
    };

    crate::config::save_credentials(&creds)?;
    Ok(creds)
}
