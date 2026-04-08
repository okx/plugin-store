/// Hyperliquid L1 chain ID (HyperEVM).
pub const CHAIN_ID: u64 = 999;

/// Arbitrum One chain ID — used for USDC deposits via the HL bridge.
pub const ARBITRUM_CHAIN_ID: u64 = 42161;

/// Hyperliquid USDC bridge contract on Arbitrum One.
/// Source: https://hyperliquid.gitbook.io/hyperliquid-docs/onboarding/how-to-start-trading
pub const HL_BRIDGE_ARBITRUM: &str = "0x2Df1c51E09aECF9cacB7bc98cB1742757f163dF7";

/// Native USDC on Arbitrum One (6 decimals).
pub const USDC_ARBITRUM: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";

/// Hyperliquid mainnet info endpoint (read-only queries).
pub fn info_url() -> &'static str {
    "https://api.hyperliquid.xyz/info"
}

/// Hyperliquid mainnet exchange endpoint (signed write operations).
pub fn exchange_url() -> &'static str {
    "https://api.hyperliquid.xyz/exchange"
}

/// Resolve a market coin symbol to its canonical uppercase form.
/// Hyperliquid uses uppercase symbols (BTC, ETH, SOL, etc.).
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
