/// Default Solana RPC endpoint (public mainnet-beta)
pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

/// Default slippage in basis points (100 = 1%)
pub const DEFAULT_SLIPPAGE_BPS: u64 = 100;

/// Fee basis points used in sell price calculation (pump.fun standard 1%)
pub const FEE_BASIS_POINTS: u64 = 100;

/// Approximate SOL threshold for bonding curve graduation (~85 SOL in lamports)
pub const GRADUATION_SOL_THRESHOLD: u64 = 85_000_000_000;
