/// Hyperliquid L1 chain ID (HyperEVM).
pub const CHAIN_ID: u64 = 999;

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
