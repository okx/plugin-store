/// Static config for the lifi-plugin: supported chains, RPC URLs, well-known token shortcuts.
///
/// Scope is intentionally limited to 6 mainstream EVM chains. LI.FI itself supports many more,
/// but onchainos / wallet integration is verified only on these. Adding a chain requires adding
/// it here AND extending plugin.yaml `api_calls` to whitelist its RPC.

pub const LIFI_API_BASE: &str = "https://li.quest/v1";

/// Standard "native gas token" sentinel used by LI.FI and most aggregators.
/// When this address appears as a token, it represents ETH (or BNB / MATIC, etc.) — the chain's
/// native asset, NOT an ERC-20. We MUST NOT call approve() on this. See knowledge base EVM-005.
pub const NATIVE_TOKEN_SENTINEL: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

/// Returns true if the address is the LI.FI / aggregator native-token sentinel (case-insensitive).
pub fn is_native_token(addr: &str) -> bool {
    addr.eq_ignore_ascii_case(NATIVE_TOKEN_SENTINEL)
}

/// One supported chain: id + canonical key + display name + public RPC.
#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub id: u64,
    pub key: &'static str,
    pub name: &'static str,
    pub rpc: &'static str,
    pub native_symbol: &'static str,
}

/// 6 supported chains. Order is the user-facing display order in `chains`.
/// Keys use community-standard short names (BASE / OP), not LI.FI's internal
/// 3-letter keys (BAS / OPT). We always pass chain IDs to the LI.FI API, so
/// our keys are display-only and should match what users actually type.
/// `parse_chain` accepts the LI.FI-style aliases (BAS / OPT) for back-compat.
pub const SUPPORTED_CHAINS: &[ChainInfo] = &[
    ChainInfo { id: 1,     key: "ETH",  name: "Ethereum", rpc: "https://ethereum-rpc.publicnode.com",     native_symbol: "ETH" },
    ChainInfo { id: 42161, key: "ARB",  name: "Arbitrum", rpc: "https://arbitrum-one-rpc.publicnode.com", native_symbol: "ETH" },
    ChainInfo { id: 8453,  key: "BASE", name: "Base",     rpc: "https://base-rpc.publicnode.com",         native_symbol: "ETH" },
    ChainInfo { id: 10,    key: "OP",   name: "Optimism", rpc: "https://optimism-rpc.publicnode.com",     native_symbol: "ETH" },
    ChainInfo { id: 56,    key: "BSC",  name: "BSC",      rpc: "https://bsc-rpc.publicnode.com",          native_symbol: "BNB" },
    ChainInfo { id: 137,   key: "POL",  name: "Polygon",  rpc: "https://polygon-bor-rpc.publicnode.com",  native_symbol: "POL" },
];

/// Look up by chain id.
pub fn chain_by_id(id: u64) -> Option<&'static ChainInfo> {
    SUPPORTED_CHAINS.iter().find(|c| c.id == id)
}

/// Look up by chain id OR canonical key (case-insensitive). Returns None if not in whitelist.
/// Numeric strings parse as ID; otherwise treated as key.
pub fn parse_chain(s: &str) -> Option<&'static ChainInfo> {
    if let Ok(id) = s.parse::<u64>() {
        return chain_by_id(id);
    }
    let upper = s.to_uppercase();
    // Allow common aliases users actually type.
    let canon = match upper.as_str() {
        "ETHEREUM" | "MAINNET" | "ETH" => "ETH",
        "ARBITRUM" | "ARB" | "ARBITRUM-ONE" => "ARB",
        "BASE" | "BAS" => "BASE",
        "OPTIMISM" | "OP" | "OPT" => "OP",
        "BSC" | "BNB" | "BINANCE" => "BSC",
        "POLYGON" | "MATIC" | "POL" => "POL",
        other => other,
    };
    SUPPORTED_CHAINS.iter().find(|c| c.key.eq_ignore_ascii_case(canon))
}

/// Pretty-print the supported list for error messages.
pub fn supported_chains_help() -> String {
    SUPPORTED_CHAINS
        .iter()
        .map(|c| format!("{} ({}, id={})", c.key, c.name, c.id))
        .collect::<Vec<_>>()
        .join(", ")
}
