// Solana chain constants
#[allow(dead_code)]
pub const SOLANA_CHAIN_ID: &str = "501";
#[allow(dead_code)]
pub const SOLANA_CHAIN_NAME: &str = "solana";
/// Native SOL placeholder address used in some DeFi protocols
#[allow(dead_code)]
pub const SOL_NATIVE_MINT: &str = "11111111111111111111111111111111";
/// Wrapped SOL mint address
#[allow(dead_code)]
pub const SOL_WRAPPED_MINT: &str = "So11111111111111111111111111111111111111112";
#[allow(dead_code)]
pub const SOL_DECIMALS: u32 = 9;
#[allow(dead_code)]
pub const USDC_SOLANA: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// Meteora DLMM constants
#[allow(dead_code)]
pub const METEORA_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const API_BASE_URL: &str = "https://dlmm.datapi.meteora.ag";

// Risk thresholds
pub const PRICE_IMPACT_WARN_THRESHOLD: f64 = 5.0;
pub const APY_RISK_WARN_THRESHOLD: f64 = 50.0;
#[allow(dead_code)]
pub const DEFAULT_SLIPPAGE_BPS: u64 = 50;
pub const DEFAULT_SLIPPAGE_PCT: f64 = 0.5; // 0.5%
