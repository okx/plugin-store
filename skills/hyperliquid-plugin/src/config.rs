/// Hyperliquid L1 chain ID (HyperEVM).
pub const CHAIN_ID: u64 = 999;

/// Arbitrum One chain ID — used for USDC deposits via the HL bridge.
pub const ARBITRUM_CHAIN_ID: u64 = 42161;

/// Hyperliquid USDC bridge contract on Arbitrum One.
pub const HL_BRIDGE_ARBITRUM: &str = "0x2Df1c51E09aECF9cacB7bc98cB1742757f163dF7";

/// Native USDC on Arbitrum One (6 decimals).
pub const USDC_ARBITRUM: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";

/// Returns true if the HYPERLIQUID_TESTNET env var is set to "1" or "true".
pub fn is_testnet() -> bool {
    matches!(
        std::env::var("HYPERLIQUID_TESTNET").as_deref(),
        Ok("1") | Ok("true")
    )
}

/// Test-only endpoint overrides must never activate from a stray production variable.
pub fn test_overrides_enabled() -> bool {
    matches!(
        std::env::var("HYPERLIQUID_TEST_MODE").as_deref(),
        Ok("1") | Ok("true")
    )
}

/// Base override for both API endpoints, read once per process.
///
/// The autotrade grant gate sits behind read-only market/position lookups, so any
/// assertion about the gate rejecting (and about nothing being signed when it does)
/// requires those lookups to resolve against a controllable host. Unset — the
/// production case — this contributes nothing and the endpoints stay byte-identical
/// to the hardcoded constants.
fn api_base_override() -> Option<&'static str> {
    static BASE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        if !test_overrides_enabled() {
            return None;
        }
        std::env::var("HYPERLIQUID_TEST_API_BASE")
            .ok()
            .map(|b| b.trim_end_matches('/').to_string())
            .filter(|b| !b.is_empty())
    })
    .as_deref()
}

/// Hyperliquid info endpoint — mainnet or testnet.
pub fn info_url() -> &'static str {
    static OVERRIDE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    if let Some(u) = OVERRIDE
        .get_or_init(|| api_base_override().map(|b| format!("{}/info", b)))
        .as_deref()
    {
        return u;
    }
    if is_testnet() {
        "https://api.hyperliquid-testnet.xyz/info"
    } else {
        "https://api.hyperliquid.xyz/info"
    }
}

/// Hyperliquid exchange endpoint — mainnet or testnet.
pub fn exchange_url() -> &'static str {
    static OVERRIDE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    if let Some(u) = OVERRIDE
        .get_or_init(|| api_base_override().map(|b| format!("{}/exchange", b)))
        .as_deref()
    {
        return u;
    }
    if is_testnet() {
        "https://api.hyperliquid-testnet.xyz/exchange"
    } else {
        "https://api.hyperliquid.xyz/exchange"
    }
}

/// Resolve a market coin symbol to its canonical uppercase form.
pub fn normalize_coin(coin: &str) -> String {
    coin.to_uppercase()
}

/// Current unix timestamp in milliseconds (used as nonce for orders).
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub const HYPER_EVM_RPC: &str = "https://rpc.hyperliquid.xyz/evm";
pub const USDC_HYPER_EVM: &str = "0x0000000000000000000000000000000000000000"; // placeholder — HyperEVM USDC contract TBD
