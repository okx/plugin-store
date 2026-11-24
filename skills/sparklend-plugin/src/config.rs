/// SparkLend (Sky Protocol) — Aave V3 fork on Ethereum Mainnet.
///
/// PoolAddressesProvider address verified against SparkLend docs:
///   https://docs.spark.fi/dev/deployments/mainnet-addresses
///
/// SparkLend is ABI-compatible with Aave V3 (same Pool interface).
/// Only Ethereum Mainnet (chain 1) is supported.
pub const CHAIN_ID: u64 = 1;
pub const CHAIN_NAME: &str = "Ethereum Mainnet";
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

/// SparkLend PoolAddressesProvider on Ethereum Mainnet.
/// This is the immutable registry entry point — the Pool proxy address
/// must always be resolved at runtime via PoolAddressesProvider.getPool().
pub const POOL_ADDRESSES_PROVIDER: &str = "0x02C3eA4e34C0cBd694D2adFa2c690EECbC1793eE";

/// WETH address on Ethereum Mainnet.
/// Used for ETH→WETH auto-wrap in supply command.
pub const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

/// Interest rate mode: variable (2). Stable rate (1) deprecated in V3.1+.
pub const INTEREST_RATE_MODE_VARIABLE: u128 = 2;

/// Aave referral code (0 = no referral)
pub const REFERRAL_CODE: u16 = 0;

/// Health factor warning threshold (human-readable)
pub const HF_WARN_THRESHOLD: f64 = 1.1;
