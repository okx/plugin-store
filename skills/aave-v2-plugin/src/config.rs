/// Static config for aave-v2-plugin: chain whitelist + Aave V2 contract addresses.
///
/// v0.1.0 scope: Aave V2 was deployed on Ethereum mainnet, Polygon, and Avalanche.
/// All 3 chains have official LendingPool + LendingPoolAddressesProvider +
/// AaveProtocolDataProvider deployments. All 3 are supported by onchainos.
///
/// V2 vs V3:
///   - V3 is the actively maintained Compound (use aave-v3-plugin for new positions)
///   - V2 is supply/borrow-active (NOT paused, unlike Compound V2). Continues to serve
///     legacy positions and protocols still integrated with V2 (Maker DSS, Yearn V2,
///     etc.). Smaller but stable TVL.
///
/// Markets are NOT hardcoded — enumerated at runtime via LendingPool.getReservesList().
/// Per-asset metadata (symbol, decimals, aToken/sDebt/vDebt addresses) fetched on demand
/// via AaveProtocolDataProvider.

#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub id: u64,
    pub key: &'static str,
    pub name: &'static str,
    pub rpc: &'static str,
    pub native_symbol: &'static str,
    /// Native gas floor for write operations (in wei). Mainnet is L1-expensive,
    /// L2/sidechains cheaper.
    pub gas_floor_wei: u128,
    /// LendingPoolAddressesProvider — root config; resolves current LendingPool /
    /// PriceOracle / etc. via getter functions. Use this if LendingPool ever migrates.
    pub addresses_provider: &'static str,
    /// LendingPool — main user-facing entry point for deposit / withdraw / borrow / repay.
    pub lending_pool: &'static str,
    /// AaveProtocolDataProvider - DEPRECATED in v0.1.0 (Ethereum mainnet PDP at the
    /// canonical address has no code; we source all data from LendingPool directly +
    /// ERC-20 calls on aToken/sDebt/vDebt addresses). Field kept for v0.2.0 if we
    /// ever route to PDP for chains where it's healthy (Polygon/Avalanche).
    pub data_provider: &'static str,
    /// WETHGateway — required for native ETH/MATIC/AVAX deposit/withdraw/borrow/repay.
    /// LendingPool itself does NOT accept native; the gateway wraps to WETH first.
    pub weth_gateway: &'static str,
    /// IncentivesController — distributes stkAAVE / WMATIC / WAVAX rewards depending
    /// on chain. Empty string if rewards not active for this chain in V2.
    pub incentives_controller: &'static str,
}

/// 3 chains: Ethereum mainnet, Polygon (PoS), Avalanche C-Chain.
/// Addresses verified on-chain via AddressesProvider.getLendingPool() etc.
pub const SUPPORTED_CHAINS: &[ChainInfo] = &[
    ChainInfo {
        id: 1,
        key: "ETH",
        name: "Ethereum",
        rpc: "https://ethereum-rpc.publicnode.com",
        native_symbol: "ETH",
        gas_floor_wei: 5_000_000_000_000_000, // 0.005 ETH (~$15) — L1 mainnet expensive
        addresses_provider:    "0xb53c1a33016b2dc6ff3153d380e0789baeb13fe1",
        lending_pool:          "0x7d2768dE32b0b80b7a3454c06BdAc94A69DDc7A9",
        data_provider:         "0x057835aDc8d6F0b9bA17f5b56C71f7Db84B16B36",
        weth_gateway:          "0xcc9a0B7c43DC2a5F023Bb9b738E45B0Ef6B06E04",
        incentives_controller: "0xd784927Ff2f95ba542BfC824c8a8a98F3495f6b5",  // stkAAVE on mainnet
    },
    ChainInfo {
        id: 137,
        key: "POLYGON",
        name: "Polygon",
        rpc: "https://polygon-bor-rpc.publicnode.com",
        native_symbol: "MATIC",
        gas_floor_wei: 100_000_000_000_000_000, // 0.1 MATIC — Polygon cheap
        addresses_provider:    "0xd05e3E715d945B59290df0ae8eF85c1BdB684744",
        lending_pool:          "0x8dFf5E27EA6b7AC08EbFdf9eB090F32ee9a30fcf",
        data_provider:         "0x7551b5D2763519d4e37e8B81929D336De671d46d",
        weth_gateway:          "0xbEadf48d62aCC944a06EEaE0A9054A90E5A7dc97",
        incentives_controller: "0x357D51124f59836DeD84c8a1730D72B749d8BC23", // WMATIC rewards
    },
    ChainInfo {
        id: 43114,
        key: "AVAX",
        name: "Avalanche",
        rpc: "https://avalanche-c-chain-rpc.publicnode.com",
        native_symbol: "AVAX",
        gas_floor_wei: 50_000_000_000_000_000, // 0.05 AVAX
        addresses_provider:    "0xb6A86025F0FE1862B372cb0ca18CE3EDe02A318f",
        lending_pool:          "0x4F01AeD16D97E3aB5ab2B501154DC9bb0F1A5A2C",
        data_provider:         "0x65285E9dfab318f57051ab2b139ccCf232945451",
        weth_gateway:          "0x8a47F74d1eE0e2edEB4F3A7e64EF3bD8e11D27C8",
        incentives_controller: "0x01D83Fe6A10D2f2B7AF17034343746188272cAc9", // WAVAX rewards
    },
];

pub fn chain_by_id(id: u64) -> Option<&'static ChainInfo> {
    SUPPORTED_CHAINS.iter().find(|c| c.id == id)
}

pub fn chain_by_key(key: &str) -> Option<&'static ChainInfo> {
    let upper = key.to_uppercase();
    let canon: &str = match upper.as_str() {
        "ETH" | "ETHEREUM" | "MAINNET" | "1" => "ETH",
        "POLYGON" | "MATIC" | "POL" | "137" => "POLYGON",
        "AVAX" | "AVALANCHE" | "C-CHAIN" | "43114" => "AVAX",
        other => other,
    };
    SUPPORTED_CHAINS.iter().find(|c| c.key == canon)
}

pub fn parse_chain(s: &str) -> Option<&'static ChainInfo> {
    if let Ok(id) = s.parse::<u64>() {
        return chain_by_id(id);
    }
    chain_by_key(s)
}

pub fn supported_chains_help() -> String {
    SUPPORTED_CHAINS
        .iter()
        .map(|c| format!("{} (id={}, native={})", c.key, c.id, c.native_symbol))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Aave V2 InterestRateMode enum (matches Pool.borrow / Pool.repay rate_mode arg).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateMode {
    /// Stable rate borrow — fixed rate that can be rebalanced by anyone if it drifts.
    /// V2 has stable rate; V3 removed stable mode entirely.
    Stable = 1,
    /// Variable rate borrow — floats with utilization curve.
    Variable = 2,
}

impl RateMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Stable),
            2 => Some(Self::Variable),
            _ => None,
        }
    }
    pub fn as_u128(self) -> u128 {
        self as u128
    }
}
