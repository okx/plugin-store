/// Beefy Finance configuration

pub const BEEFY_API_BASE: &str = "https://api.beefy.finance";

/// Chain ID to Beefy chain name mapping
pub fn chain_id_to_beefy_name(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("ethereum"),
        56 => Some("bsc"),
        137 => Some("polygon"),
        250 => Some("fantom"),
        43114 => Some("avax"),
        42161 => Some("arbitrum"),
        10 => Some("optimism"),
        8453 => Some("base"),
        324 => Some("zksync"),
        _ => None,
    }
}

/// RPC endpoints per chain
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "https://ethereum.publicnode.com",
        56 => "https://bsc-rpc.publicnode.com",
        137 => "https://polygon-bor-rpc.publicnode.com",
        42161 => "https://arbitrum-one-rpc.publicnode.com",
        10 => "https://optimism-rpc.publicnode.com",
        8453 => "https://base-rpc.publicnode.com",
        _ => "https://base-rpc.publicnode.com",
    }
}
