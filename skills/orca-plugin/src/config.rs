// Solana / Orca constants
pub const SOLANA_CHAIN_ID: &str = "501";
pub const SOLANA_CHAIN_NAME: &str = "solana";

// Native SOL system program address (for balance queries)
pub const SOL_NATIVE_MINT: &str = "11111111111111111111111111111111";

// Wrapped SOL mint (used for DEX swaps involving SOL)
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

pub const SOL_DECIMALS: u32 = 9;

// Well-known token mints on Solana mainnet
pub const USDC_SOLANA: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_SOLANA: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
pub const ORCA_TOKEN: &str = "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE";

// Orca Whirlpool program address
pub const WHIRLPOOL_PROGRAM: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

// Orca REST API base URL (v1 — the publicly working endpoint)
pub const ORCA_API_BASE: &str = "https://api.orca.so/v1";

// Solana public RPC endpoint
pub const SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

// Default configuration
pub const DEFAULT_SLIPPAGE_BPS: u64 = 50; // 0.5%
pub const DEFAULT_MIN_POOL_TVL_USD: f64 = 10_000.0;
pub const PRICE_IMPACT_WARN_THRESHOLD: f64 = 2.0; // percent
pub const PRICE_IMPACT_BLOCK_THRESHOLD: f64 = 10.0; // percent
