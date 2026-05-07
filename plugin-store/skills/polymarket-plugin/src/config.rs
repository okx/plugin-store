use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;


/// Trading mode: which wallet acts as the order maker.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TradingMode {
    /// EOA mode: the onchainos wallet is the maker. Requires POL for gas on every approve.
    #[default]
    Eoa,
    /// PolyProxy mode: a Polymarket proxy contract is the maker. No POL needed for trading;
    /// Polymarket's relayer covers gas. Requires USDC.e deposited into the proxy wallet.
    PolyProxy,
    /// DepositWallet mode: a Polymarket ERC-1967 deposit wallet is the maker.
    /// Gasless (relayer-paid). maker = signer = deposit_wallet_address.
    /// Signature type 3 (POLY_1271 / ERC-1271). New users from v0.6.0 onwards.
    DepositWallet,
}

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
    /// Polymarket proxy wallet address (maker for PolyProxy orders).
    #[serde(default)]
    pub proxy_wallet: Option<String>,
    /// Polymarket deposit wallet address (maker for DepositWallet orders).
    /// ERC-1967 proxy deployed by DEPOSIT_WALLET_FACTORY. New users from v0.6.0.
    #[serde(default)]
    pub deposit_wallet: Option<String>,
    /// Active trading mode. Defaults to EOA if not set (backwards-compatible).
    #[serde(default)]
    pub mode: TradingMode,
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

// ── Multi-wallet credential store ─────────────────────────────────────────────
//
// v2 on-disk format:
//   {
//     "_version": 2,
//     "0xabc...": { "api_key": "...", "mode": "poly_proxy", ... },
//     "0xdef...": { "api_key": "...", "mode": "deposit_wallet", ... }
//   }
//
// v1 (legacy) format: flat Credentials object with a "signing_address" field.
// Auto-migrated to v2 on first read — user sees no interruption.

/// Per-wallet entry in the multi-wallet store (address is the map key).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CredentialsEntry {
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
    #[serde(default)]
    pub nonce: u64,
    #[serde(default)]
    pub proxy_wallet: Option<String>,
    #[serde(default)]
    pub deposit_wallet: Option<String>,
    #[serde(default)]
    pub mode: TradingMode,
}

impl CredentialsEntry {
    fn is_empty(&self) -> bool { self.api_key.is_empty() }
    fn into_credentials(self, addr: &str) -> Credentials {
        Credentials {
            api_key: self.api_key,
            secret: self.secret,
            passphrase: self.passphrase,
            nonce: self.nonce,
            signing_address: addr.to_lowercase(),
            proxy_wallet: self.proxy_wallet,
            deposit_wallet: self.deposit_wallet,
            mode: self.mode,
        }
    }
}

impl From<&Credentials> for CredentialsEntry {
    fn from(c: &Credentials) -> Self {
        CredentialsEntry {
            api_key: c.api_key.clone(),
            secret: c.secret.clone(),
            passphrase: c.passphrase.clone(),
            nonce: c.nonce,
            proxy_wallet: c.proxy_wallet.clone(),
            deposit_wallet: c.deposit_wallet.clone(),
            mode: c.mode.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct MultiWalletStore {
    #[serde(rename = "_version")]
    version: u32,
    #[serde(flatten)]
    wallets: HashMap<String, CredentialsEntry>,
}

fn warn_permissions(path: &PathBuf) {
    #[cfg(unix)]
    if let Ok(meta) = std::fs::metadata(path) {
        let mode = meta.permissions().mode();
        if mode & 0o077 != 0 {
            eprintln!(
                "[polymarket] Warning: {} has loose permissions ({:o}). Run: chmod 600 {}",
                path.display(), mode & 0o777, path.display()
            );
        }
    }
}

/// Load the multi-wallet store from disk, auto-migrating v1 format if needed.
fn load_store() -> Result<MultiWalletStore> {
    let path = creds_path();
    if !path.exists() {
        return Ok(MultiWalletStore { version: 2, wallets: HashMap::new() });
    }
    warn_permissions(&path);
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| "parsing creds.json")?;

    if v.get("_version").and_then(|x| x.as_u64()) == Some(2) {
        // v2 multi-wallet format
        serde_json::from_value(v).with_context(|| "parsing multi-wallet creds.json")
    } else if v.get("signing_address").is_some() {
        // v1 legacy format — auto-migrate transparently
        let old: Credentials = serde_json::from_value(v)
            .with_context(|| "parsing legacy creds.json")?;
        let mut store = MultiWalletStore { version: 2, wallets: HashMap::new() };
        if !old.is_empty() {
            let addr = old.signing_address.to_lowercase();
            store.wallets.insert(addr, CredentialsEntry::from(&old));
        }
        eprintln!("[polymarket] Migrated creds.json to multi-wallet format (v2).");
        save_store(&store)?;
        Ok(store)
    } else {
        Ok(MultiWalletStore { version: 2, wallets: HashMap::new() })
    }
}

/// Write the multi-wallet store to disk with 0600 permissions.
fn save_store(store: &MultiWalletStore) -> Result<()> {
    let path = creds_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, &data)
        .with_context(|| format!("writing {}", path.display()))?;
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", path.display()))?;
    Ok(())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Load credentials for a specific wallet address.
/// Returns None if no entry exists for that address.
/// Auto-migrates v1 single-wallet format on first read.
pub fn load_credentials_for(addr: &str) -> Result<Option<Credentials>> {
    let store = load_store()?;
    Ok(store.wallets.get(&addr.to_lowercase())
        .filter(|e| !e.is_empty())
        .map(|e| e.clone().into_credentials(addr)))
}

/// Load any stored credentials (first entry). Used by callers that don't yet
/// know the wallet address (hint text, redeem proxy detection, etc.).
/// Prefer load_credentials_for(addr) for exact wallet lookups.
pub fn load_credentials() -> Result<Option<Credentials>> {
    let store = load_store()?;
    Ok(store.wallets.into_iter()
        .find(|(_, e)| !e.is_empty())
        .map(|(addr, e)| e.into_credentials(&addr)))
}

/// Save (upsert) credentials for the wallet in creds.signing_address.
/// Creates or updates the entry for that address; other wallets are untouched.
pub fn save_credentials(creds: &Credentials) -> Result<()> {
    anyhow::ensure!(!creds.signing_address.is_empty(), "save_credentials: signing_address must be set");
    let mut store = load_store()?;
    store.wallets.insert(creds.signing_address.to_lowercase(), CredentialsEntry::from(creds));
    save_store(&store)
}

/// Remove credentials for a specific wallet address (forces re-derivation on next use).
/// Other wallet entries are preserved.
pub fn clear_credentials_for(addr: &str) -> Result<()> {
    let mut store = load_store()?;
    store.wallets.remove(&addr.to_lowercase());
    save_store(&store)
}

/// Clear ALL stored credentials (full reset — for unrecoverable auth errors).
pub fn clear_credentials() -> Result<()> {
    let path = creds_path();
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

/// CLOB order version — determines which exchange contract and EIP-712 struct to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderVersion {
    /// Original exchange (0x4bFb41...). EIP-712 domain version "1".
    V1,
    /// New exchange released 2026-04-21 (0xE11118...). EIP-712 domain version "2".
    V2,
}

/// Contract addresses on Polygon (chain 137)
pub struct Contracts;

impl Contracts {
    // ── V1 exchange contracts (legacy) ────────────────────────────────────────
    pub const CTF_EXCHANGE: &'static str = "0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E";
    pub const NEG_RISK_CTF_EXCHANGE: &'static str = "0xC5d563A36AE78145C45a50134d48A1215220f80a";

    // ── V2 exchange contracts (released 2026-04-21) ───────────────────────────
    pub const CTF_EXCHANGE_V2: &'static str = "0xE111180000d2663C0091e4f400237545B87B996B";
    pub const NEG_RISK_CTF_EXCHANGE_V2: &'static str = "0xe2222d279d744050d28e00520010520000310F59";

    // ── Shared / unchanged contracts ──────────────────────────────────────────
    pub const NEG_RISK_ADAPTER: &'static str = "0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296";
    pub const CTF: &'static str = "0x4D97DCd97eC945f40cF65F87097ACe5EA0476045";
    pub const USDC_E: &'static str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
    /// Polymarket USD — replaces USDC.e as collateral for V2 exchange contracts (live ~2026-04-28).
    pub const PUSD: &'static str = "0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB";
    /// Collateral Onramp: wrap(address _asset, address _to, uint256 _amount) USDC.e → pUSD.
    pub const COLLATERAL_ONRAMP: &'static str = "0x93070a847efEf7F70739046A929D47a521F5B8ee";
    pub const PROXY_FACTORY: &'static str = "0xaB45c5A4B0c941a2F231C04C3f49182e1A254052";
    pub const GNOSIS_SAFE_FACTORY: &'static str = "0xaacfeea03eb1561c4e67d661e40682bd20e3541b";
    /// Deposit wallet factory — ERC-1967 proxies, one per user. Deployed via relayer WALLET-CREATE.
    pub const DEPOSIT_WALLET_FACTORY: &'static str = "0x00000000000Fb5C9ADea0298D729A0CB3823Cc07";
    pub const UMA_ADAPTER: &'static str = "0x6A9D222616C90FcA5754cd1333cFD9b7fb6a4F74";

    /// Return the V1 exchange address for the given market type.
    pub fn exchange_for(neg_risk: bool) -> &'static str {
        if neg_risk { Self::NEG_RISK_CTF_EXCHANGE } else { Self::CTF_EXCHANGE }
    }

    /// Return the V2 exchange address for the given market type.
    pub fn exchange_for_v2(neg_risk: bool) -> &'static str {
        if neg_risk { Self::NEG_RISK_CTF_EXCHANGE_V2 } else { Self::CTF_EXCHANGE_V2 }
    }

    /// Return the exchange address for the given version and market type.
    pub fn exchange(version: OrderVersion, neg_risk: bool) -> &'static str {
        match version {
            OrderVersion::V1 => Self::exchange_for(neg_risk),
            OrderVersion::V2 => Self::exchange_for_v2(neg_risk),
        }
    }
}

/// Base URLs
pub struct Urls;

impl Urls {
    pub const CLOB: &'static str = "https://clob.polymarket.com";
    pub const GAMMA: &'static str = "https://gamma-api.polymarket.com";
    pub const DATA: &'static str = "https://data-api.polymarket.com";
    pub const BRIDGE: &'static str = "https://bridge.polymarket.com";
    pub const RELAYER: &'static str = "https://relayer-v2.polymarket.com";
    pub const POLYGON_RPC:  &'static str = "https://polygon.drpc.org";
    /// Dedicated Polygon RPC for eth_getLogs event scanning.
    /// publicnode supports ≤7,998 block range per request (drpc free tier rejects eth_getLogs above ~7,500).
    pub const POLYGON_LOGS_RPC: &'static str = "https://polygon-bor-rpc.publicnode.com";
    pub const ETHEREUM_RPC: &'static str = "https://ethereum.publicnode.com";
    pub const ARBITRUM_RPC: &'static str = "https://arbitrum.drpc.org";
    pub const BASE_RPC:     &'static str = "https://base.drpc.org";
    pub const OPTIMISM_RPC: &'static str = "https://optimism.drpc.org";
    pub const BNB_RPC:      &'static str = "https://bsc.publicnode.com";

    // ── Env-var-overridable accessors ────────────────────────────────────────
    //
    // These are used in place of the const fields throughout the codebase so
    // that integration tests can redirect HTTP traffic to local mock servers
    // by setting the corresponding POLYMARKET_TEST_* env vars.
    //
    // Production code never sets these vars, so the const defaults always apply
    // in normal operation.

    pub fn polygon_rpc() -> String {
        std::env::var("POLYMARKET_TEST_POLYGON_RPC")
            .unwrap_or_else(|_| Self::POLYGON_RPC.to_string())
    }

    /// RPC endpoint used for eth_getLogs event scanning.
    /// Uses publicnode (supports ≤7,998 block range) instead of drpc free tier.
    pub fn polygon_logs_rpc() -> String {
        std::env::var("POLYMARKET_TEST_POLYGON_RPC")
            .unwrap_or_else(|_| Self::POLYGON_LOGS_RPC.to_string())
    }

    pub fn clob() -> String {
        std::env::var("POLYMARKET_TEST_CLOB_URL")
            .unwrap_or_else(|_| Self::CLOB.to_string())
    }

    pub fn gamma() -> String {
        std::env::var("POLYMARKET_TEST_GAMMA_URL")
            .unwrap_or_else(|_| Self::GAMMA.to_string())
    }

    pub fn data() -> String {
        std::env::var("POLYMARKET_TEST_DATA_URL")
            .unwrap_or_else(|_| Self::DATA.to_string())
    }
}
