use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;


/// Persisted API credentials derived via L1 (ClobAuth EIP-712) auth.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Credentials {
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
    pub nonce: u64,
    /// Ethereum address of the onchainos wallet used to derive these credentials.
    #[serde(default)]
    pub signing_address: String,
    /// Polymarket proxy wallet address (maker for orders). Populated after polymarket.com onboarding.
    #[serde(default)]
    pub proxy_wallet: Option<String>,
}

impl Credentials {
    pub fn is_empty(&self) -> bool {
        self.api_key.is_empty()
    }
}

fn creds_path() -> PathBuf {
    // Always use ~/.config/polymarket/creds.json per spec, regardless of platform.
    // dirs::config_dir() returns ~/Library/Application Support on macOS which diverges from spec.
    let base = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config");
    base.join("polymarket").join("creds.json")
}

pub fn load_credentials() -> Result<Option<Credentials>> {
    let path = creds_path();
    if !path.exists() {
        return Ok(None);
    }
    // Warn if file is readable by group/other (Unix only)
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(&path) {
        let mode = meta.permissions().mode();
        if mode & 0o077 != 0 {
            eprintln!(
                "[polymarket] Warning: {} has loose permissions ({:o}). Run: chmod 600 {}",
                path.display(), mode & 0o777, path.display()
            );
        }
    }
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let creds: Credentials = serde_json::from_str(&data)
        .with_context(|| "parsing creds.json")?;
    if creds.is_empty() {
        return Ok(None);
    }
    Ok(Some(creds))
}

pub fn save_credentials(creds: &Credentials) -> Result<()> {
    let path = creds_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(creds)?;
    std::fs::write(&path, &data)
        .with_context(|| format!("writing {}", path.display()))?;
    // Restrict to owner read/write only (Unix only — Windows uses ACLs)
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", path.display()))?;
    Ok(())
}

/// Contract addresses on Polygon (chain 137)
pub struct Contracts;

impl Contracts {
    pub const CTF_EXCHANGE: &'static str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
    pub const NEG_RISK_CTF_EXCHANGE: &'static str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";
    pub const NEG_RISK_ADAPTER: &'static str = "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296";
    pub const CTF: &'static str = "0x4D97DCd97eC945f40cF65F87097ACe5EA0476045";
    pub const USDC_E: &'static str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
    pub const PROXY_FACTORY: &'static str = "0xaB45c5A4B0c941a2F231C04C3f49182e1A254052";
    pub const GNOSIS_SAFE_FACTORY: &'static str = "0xaacfeea03eb1561c4e67d661e40682bd20e3541b";
    pub const UMA_ADAPTER: &'static str = "0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74";

    pub fn exchange_for(neg_risk: bool) -> &'static str {
        if neg_risk {
            Self::NEG_RISK_CTF_EXCHANGE
        } else {
            Self::CTF_EXCHANGE
        }
    }
}

/// Base URLs
pub struct Urls;

impl Urls {
    pub const CLOB: &'static str = "https://clob.polymarket.com";
    pub const GAMMA: &'static str = "https://gamma-api.polymarket.com";
    pub const DATA: &'static str = "https://data-api.polymarket.com";
}
