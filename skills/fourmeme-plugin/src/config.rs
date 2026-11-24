/// Chain + Four.meme contract addresses for fourmeme-plugin.
///
/// v0.1 supports BSC mainnet (chain 56) only. The Helper3 contract is also deployed
/// on Arbitrum (`0x02287dc3CcA964a025DAaB1111135A46C10D3A57`) and Base
/// (`0x1172FABbAc4Fe05f5a5Cebd8EBBC593A76c42399`), but TokenManager V2 — where the
/// actual buy/sell flow lives — is BSC-only, so we don't expose the multi-chain
/// surface yet.

pub struct Urls;

impl Urls {
    pub const BSC_RPC: &'static str = "https://bsc-rpc.publicnode.com";

    /// Test-overridable RPC. Production uses the public node; tests inject a mock.
    pub fn rpc_for_chain(chain_id: u64) -> Option<String> {
        match chain_id {
            56 => Some(
                std::env::var("FOURMEME_TEST_BSC_RPC")
                    .unwrap_or_else(|_| Self::BSC_RPC.to_string()),
            ),
            _ => None,
        }
    }
}

pub mod addresses {
    /// TokenManager V2 — the live factory + buy/sell router for tokens launched 2024+.
    /// Pre-graduate tokens (those still on the bonding curve) trade through this proxy.
    pub const TOKEN_MANAGER_V2: &str = "0x5c952063c7fc8610FFDB798152D69F0B9550762b";

    /// TokenManager V1 — legacy proxy. Some older tokens were launched here. Helper3
    /// transparently routes to V1 or V2 based on which manager owns the token.
    pub const TOKEN_MANAGER_V1: &str = "0xEC4549caDcE5DA21Df6E6422d448034B5233bFbC";

    /// TokenManagerHelper3 — unified read/quote contract on BSC. Use for getTokenInfo,
    /// tryBuy, trySell. Returns the *actual* TokenManager (V1 or V2) that should
    /// receive the buy/sell tx via `tokenManager` field.
    pub const TOKEN_MANAGER_HELPER3: &str = "0xF251F83e40a78868FcfA3FA4599Dad6494E46034";
}

pub const SUPPORTED_CHAINS: &[(u64, &str)] = &[(56, "bsc")];

pub fn chain_name(chain_id: u64) -> Option<&'static str> {
    SUPPORTED_CHAINS
        .iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, n)| *n)
}

pub fn is_supported_chain(chain_id: u64) -> bool {
    SUPPORTED_CHAINS.iter().any(|(id, _)| *id == chain_id)
}

/// All Four.meme tokens use 18 decimals (verified by querying live tokens on BSC).
pub const TOKEN_DECIMALS: u32 = 18;

/// BNB sentinel — `quote == 0x0` from getTokenInfo means the token is BNB-quoted.
pub const NATIVE_QUOTE: &str = "0x0000000000000000000000000000000000000000";
