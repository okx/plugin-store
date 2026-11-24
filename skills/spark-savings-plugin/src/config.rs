/// Static config for spark-savings-plugin: chains, contract addresses, mechanism per chain.
///
/// Three chains supported in v0.1.0:
///   - Ethereum: full ERC-4626 deposit/redeem on sUSDS contract
///   - Base / Arbitrum: deposit/redeem via Spark PSM (cross-chain sUSDS is NOT a vault)
///
/// Adding a chain requires extending plugin.yaml `api_calls` to whitelist its RPC.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertMechanism {
    /// Ethereum mainnet: native ERC-4626 vault. deposit(assets, receiver) / redeem(shares, receiver, owner).
    Erc4626Vault,
    /// L2: bridged sUSDS, no vault. Conversion goes through Spark PSM contract.
    SparkPsm,
}

#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub id: u64,
    pub key: &'static str,
    pub name: &'static str,
    pub rpc: &'static str,
    pub native_symbol: &'static str,
    pub usds: &'static str,
    pub susds: &'static str,
    /// Ethereum mainnet: legacy DAI ERC-20. Other chains: None.
    pub dai: Option<&'static str>,
    /// Ethereum mainnet only: DaiUsds migrator contract. Other chains: None.
    pub dai_usds_migrator: Option<&'static str>,
    /// Spark PSM address. Ethereum: None (uses ERC-4626 directly). Base/Arbitrum: PSM addr.
    pub spark_psm: Option<&'static str>,
    pub mechanism: ConvertMechanism,
}

/// 3 supported chains in v0.1.0.
///
/// Sources:
///   - sUSDS / USDS Ethereum:        https://docs.spark.fi/dev/savings/susds-token
///   - sUSDS / USDS L2 deployments:  https://docs.spark.fi/dev/savings/cross-chain-usds-and-susds
///   - DaiUsds migrator (Ethereum):  https://etherscan.io/address/0x3225737a9Bbb6473CB4a45b7244ACa2BeFdB276A
///   - Spark PSM Base / Arbitrum:    https://docs.spark.fi/dev/savings/spark-psm
pub const SUPPORTED_CHAINS: &[ChainInfo] = &[
    ChainInfo {
        id: 1,
        key: "ETH",
        name: "Ethereum",
        rpc: "https://ethereum-rpc.publicnode.com",
        native_symbol: "ETH",
        usds:  "0xdC035D45d973E3EC169d2276DDab16f1e407384F",
        susds: "0xa3931d71877c0e7a3148cb7eb4463524fec27fbd",
        dai:                Some("0x6B175474E89094C44Da98b954EedeAC495271d0F"),
        dai_usds_migrator:  Some("0x3225737a9Bbb6473CB4a45b7244ACa2BeFdB276A"),
        spark_psm:          None,
        mechanism: ConvertMechanism::Erc4626Vault,
    },
    ChainInfo {
        id: 8453,
        key: "BASE",
        name: "Base",
        rpc: "https://base-rpc.publicnode.com",
        native_symbol: "ETH",
        usds:  "0x820C137fa70C8691f0e44Dc420a5e53c168921Dc",
        susds: "0x5875eEE11Cf8398102FdAd704C9E96607675467a",
        dai:                None,
        dai_usds_migrator:  None,
        spark_psm:          Some("0x1601843c5E9bC251A3272907010AFa41Fa18347E"),
        mechanism: ConvertMechanism::SparkPsm,
    },
    ChainInfo {
        id: 42161,
        key: "ARB",
        name: "Arbitrum",
        rpc: "https://arbitrum-one-rpc.publicnode.com",
        native_symbol: "ETH",
        usds:  "0x6491c05A82219b8D1479057361ff1654749b876b",
        susds: "0xdDb46999F8891663a8F2828d25298f70416d7610",
        dai:                None,
        dai_usds_migrator:  None,
        spark_psm:          Some("0x2B05F8e1cACC6974fD79A673a341Fe1f58d27266"),
        mechanism: ConvertMechanism::SparkPsm,
    },
];

pub fn chain_by_id(id: u64) -> Option<&'static ChainInfo> {
    SUPPORTED_CHAINS.iter().find(|c| c.id == id)
}

/// Look up by chain id OR canonical key (case-insensitive).
/// Numeric strings parse as ID; otherwise treated as key.
pub fn parse_chain(s: &str) -> Option<&'static ChainInfo> {
    if let Ok(id) = s.parse::<u64>() {
        return chain_by_id(id);
    }
    let upper = s.to_uppercase();
    let canon = match upper.as_str() {
        "ETHEREUM" | "MAINNET" | "ETH" => "ETH",
        "BASE" | "BAS" => "BASE",
        "ARBITRUM" | "ARB" | "ARBITRUM-ONE" => "ARB",
        other => other,
    };
    SUPPORTED_CHAINS.iter().find(|c| c.key.eq_ignore_ascii_case(canon))
}

pub fn supported_chains_help() -> String {
    SUPPORTED_CHAINS
        .iter()
        .map(|c| format!("{} ({}, id={})", c.key, c.name, c.id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// USDS / sUSDS / DAI all use 18 decimals across every chain. This constant
/// avoids decimal-resolution RPC roundtrips for known tokens.
pub const STABLE_DECIMALS: u32 = 18;
