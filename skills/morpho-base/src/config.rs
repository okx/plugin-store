/// Chain configuration for Morpho V1 on Base.

pub struct ChainConfig {
    pub chain_id: u64,
    pub rpc_url: &'static str,
    pub morpho_blue: &'static str,
    pub merkl_distributor: &'static str,
}

/// Morpho Blue is deployed at the same address on Base as Ethereum.
pub const CHAIN_BASE: ChainConfig = ChainConfig {
    chain_id: 8453,
    rpc_url: "https://base-rpc.publicnode.com",
    morpho_blue: "0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb",
    merkl_distributor: "0x3Ef3D8bA38EBe18DB133cEc108f4D14CE00Dd9Ae",
};

pub const GRAPHQL_URL: &str = "https://blue-api.morpho.org/graphql";
pub const MERKL_API_URL: &str = "https://api.merkl.xyz";

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        8453 => Ok(&CHAIN_BASE),
        _ => anyhow::bail!("Unsupported chain ID: {}. morpho-base only supports Base (8453)", chain_id),
    }
}

pub fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        8453 => "Base",
        _ => "Unknown",
    }
}
