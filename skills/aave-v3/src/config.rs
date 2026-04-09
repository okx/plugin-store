/// Per-chain configuration for Aave V3.
///
/// POOL_ADDRESSES_PROVIDER addresses are the immutable registry entry points —
/// safe to store in config. Pool address itself must ALWAYS be resolved at
/// runtime via PoolAddressesProvider.getPool().
///
/// Addresses verified against BGD Labs aave-address-book:
///   - Ethereum: https://github.com/bgd-labs/aave-address-book/blob/main/src/AaveV3Ethereum.sol
///   - Polygon:  https://github.com/bgd-labs/aave-address-book/blob/main/src/AaveV3Polygon.sol
///   - Arbitrum: https://github.com/bgd-labs/aave-address-book/blob/main/src/AaveV3Arbitrum.sol
///   - Base:     https://github.com/bgd-labs/aave-address-book/blob/main/src/AaveV3Base.sol
///
/// Note: Polygon (137) and Arbitrum (42161) intentionally share the same
/// PoolAddressesProvider address (0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb).
/// This is correct per BGD Labs address book — both chains deploy to the same
/// address due to Aave's cross-chain deterministic deployment pattern.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub pool_addresses_provider: &'static str,
    pub rpc_url: &'static str,
    pub name: &'static str,
}

pub static CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 1,
        pool_addresses_provider: "0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e",
        rpc_url: "https://ethereum.publicnode.com",
        name: "Ethereum Mainnet",
    },
    ChainConfig {
        chain_id: 137,
        pool_addresses_provider: "0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb",
        rpc_url: "https://polygon-bor-rpc.publicnode.com",
        name: "Polygon",
    },
    ChainConfig {
        chain_id: 42161,
        pool_addresses_provider: "0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb",
        rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        name: "Arbitrum One",
    },
    ChainConfig {
        chain_id: 8453,
        pool_addresses_provider: "0xe20fCBdBfFC4Dd138cE8b2E6FBb6CB49777ad64D",
        rpc_url: "https://base-rpc.publicnode.com",
        name: "Base",
    },
];

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    CHAINS
        .iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported chain ID: {}. Supported chains: {}",
                chain_id,
                CHAINS
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.chain_id))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// Interest rate mode constants
pub const INTEREST_RATE_MODE_VARIABLE: u128 = 2;
/// Stable rate (deprecated in V3.1+) — blocked in borrow command
#[allow(dead_code)]
pub const INTEREST_RATE_MODE_STABLE: u128 = 1;

/// Aave referral code (0 = no referral)
pub const REFERRAL_CODE: u16 = 0;

/// Health factor thresholds (scaled 1e18 on-chain, these are human-readable)
pub const HF_WARN_THRESHOLD: f64 = 1.1;
#[allow(dead_code)]
pub const HF_DANGER_THRESHOLD: f64 = 1.05;

