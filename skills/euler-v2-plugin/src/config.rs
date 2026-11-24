/// Chain configuration for Euler v2 plugin.
///
/// Address book is fetched dynamically from `app.euler.finance/api/euler-chains`,
/// so this module only holds default RPC URLs (env-overridable for tests) and the
/// list of chain IDs we ship support for in v0.1.

pub struct Urls;

impl Urls {
    pub const EULER_API: &'static str = "https://app.euler.finance";

    pub const ETHEREUM_RPC: &'static str = "https://ethereum-rpc.publicnode.com";
    pub const BASE_RPC:     &'static str = "https://base-rpc.publicnode.com";
    pub const ARBITRUM_RPC: &'static str = "https://arbitrum.drpc.org";

    /// Test-overridable accessors. Production reads the env var; tests inject mock URLs.
    pub fn euler_api() -> String {
        std::env::var("EULER_TEST_API_URL")
            .unwrap_or_else(|_| Self::EULER_API.to_string())
    }

    pub fn rpc_for_chain(chain_id: u64) -> Option<String> {
        match chain_id {
            1     => Some(std::env::var("EULER_TEST_ETHEREUM_RPC").unwrap_or_else(|_| Self::ETHEREUM_RPC.to_string())),
            8453  => Some(std::env::var("EULER_TEST_BASE_RPC").unwrap_or_else(|_| Self::BASE_RPC.to_string())),
            42161 => Some(std::env::var("EULER_TEST_ARBITRUM_RPC").unwrap_or_else(|_| Self::ARBITRUM_RPC.to_string())),
            _     => None,
        }
    }
}

/// Chains supported in v0.1. Adding a new chain only requires:
///   1. Adding it here
///   2. Adding the RPC URL to `rpc_for_chain`
///   3. Adding the RPC domain to plugin.yaml `api_calls`
/// All contract addresses come from `/api/euler-chains`.
pub const SUPPORTED_CHAINS: &[(u64, &str)] = &[
    (1,     "ethereum"),
    (8453,  "base"),
    (42161, "arbitrum"),
];

pub fn chain_name(chain_id: u64) -> Option<&'static str> {
    SUPPORTED_CHAINS.iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, name)| *name)
}

pub fn is_supported_chain(chain_id: u64) -> bool {
    SUPPORTED_CHAINS.iter().any(|(id, _)| *id == chain_id)
}
