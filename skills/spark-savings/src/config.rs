/// Per-chain configuration for Spark Savings.
///
/// On Ethereum (L1): sUSDS and sDAI are ERC-4626 vaults.
/// On L2s (Base, Arbitrum, Optimism): sUSDS is a bridged ERC-20; deposits/withdrawals
/// use the Spark PSM3 contract which swaps USDS <-> sUSDS.
///
/// Addresses sourced from:
///   https://github.com/sparkdotfi/spark-address-registry
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub rpc_url: &'static str,
    pub name: &'static str,
    /// sUSDS token address (ERC-4626 on L1, bridged ERC-20 on L2)
    pub susds: &'static str,
    /// sDAI token address (ERC-4626, Ethereum only — None on L2)
    pub sdai: Option<&'static str>,
    /// USDS token address
    pub usds: &'static str,
    /// DAI token address (Ethereum only)
    pub dai: Option<&'static str>,
    /// Spark PSM3 address (L2 only — None on Ethereum)
    pub psm3: Option<&'static str>,
    /// SSR Auth Oracle (L2 only, for reading SSR rate on-chain)
    pub ssr_oracle: Option<&'static str>,
    /// Whether this chain uses PSM3 (L2) or direct ERC-4626 (L1)
    pub use_psm3: bool,
}

pub static CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 1,
        rpc_url: "https://ethereum.publicnode.com",
        name: "Ethereum Mainnet",
        susds: "0xa3931d71877C0E7a3148CB7Eb4463524FEc27fbD",
        sdai: Some("0x83F20F44975D03b1b09e64809B757c47f942BEeA"),
        usds: "0xdC035D45d973E3EC169d2276DDab16f1e407384F",
        dai: Some("0x6B175474E89094C44Da98b954EedeAC495271d0F"),
        psm3: None,
        ssr_oracle: None,
        use_psm3: false,
    },
    ChainConfig {
        chain_id: 8453,
        rpc_url: "https://base-rpc.publicnode.com",
        name: "Base",
        susds: "0x5875eEE11Cf8398102FdAd704C9E96607675467a",
        sdai: None,
        usds: "0x820C137fa70C8691f0e44Dc420a5e53c168921Dc",
        dai: None,
        psm3: Some("0x1601843c5E9bC251A3272907010AFa41Fa18347E"),
        ssr_oracle: Some("0x65d946e533748A998B1f0E430803e39A6388f7a1"),
        use_psm3: true,
    },
    ChainConfig {
        chain_id: 42161,
        rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        name: "Arbitrum One",
        susds: "0xdDb46999F8891663a8F2828d25298f70416d7610",
        sdai: None,
        usds: "0x6491c05a82219b8d1479057361ff1654749b876b",
        dai: None,
        psm3: Some("0x2B05F8e1cACC6974fD79A673a341Fe1f58d27266"),
        ssr_oracle: Some("0xEE2816c1E1eed14d444552654Ed3027abC033A36"),
        use_psm3: true,
    },
    ChainConfig {
        chain_id: 10,
        rpc_url: "https://optimism.publicnode.com",
        name: "Optimism",
        susds: "0xb5B2dc7fd34C249F4be7fB1fCea07950784229e0",
        sdai: None,
        usds: "0x4F13a96EC5C4Cf34e442b46Bbd98a0791F20edC3",
        dai: None,
        psm3: Some("0xe0F9978b907853F354d79188A3dEfbD41978af62"),
        ssr_oracle: Some("0x6E53585449142A5E6D5fC918AE6BEa341dC81C68"),
        use_psm3: true,
    },
];

/// Ethereum MakerDAO Pot contract (for DSR)
pub const MAKER_POT: &str = "0x197E90f9FAD81970bA7976f33CbD77088E5D7cf7";

/// Ethereum RPC for cross-chain rate reads
pub const ETHEREUM_RPC: &str = "https://ethereum.publicnode.com";

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

#[allow(dead_code)]
pub fn chain_id_to_name(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "ethereum",
        8453 => "base",
        42161 => "arbitrum",
        10 => "optimism",
        _ => "ethereum",
    }
}
