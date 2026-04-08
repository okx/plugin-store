// Configuration constants for Instadapp Lite vaults

/// Ethereum mainnet RPC — use publicnode to avoid rate limits
pub const ETHEREUM_RPC: &str = "https://ethereum.publicnode.com";

/// Instadapp Lite ETH v1 vault (iETH) — accepts native ETH via supplyEth()
pub const IETH_V1_VAULT: &str = "0xc383a3833A87009fD9597F8184979AF5eDFad019";

/// Instadapp Lite ETH v2 vault (iETHv2) — ERC-4626, accepts stETH deposits
pub const IETH_V2_VAULT: &str = "0xa0d3707c569ff8c87fa923d3823ec5d81c98be78";

/// stETH (Lido Staked ETH) — underlying asset for iETHv2
pub const STETH_ADDRESS: &str = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84";

/// WETH — can also be supplied via supply(token,amount,to)
pub const WETH_ADDRESS: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

/// Vault info
pub struct VaultInfo {
    pub address: &'static str,
    pub name: &'static str,
    pub symbol: &'static str,
    pub version: &'static str,
    pub underlying_symbol: &'static str,
    pub decimals: u32,
}

pub const VAULTS: &[VaultInfo] = &[
    VaultInfo {
        address: IETH_V1_VAULT,
        name: "Instadapp ETH",
        symbol: "iETH",
        version: "v1",
        underlying_symbol: "ETH",
        decimals: 18,
    },
    VaultInfo {
        address: IETH_V2_VAULT,
        name: "Instadapp ETH v2",
        symbol: "iETHv2",
        version: "v2",
        underlying_symbol: "stETH",
        decimals: 18,
    },
];

/// Resolve vault address or symbol to address
/// Accepts: "v1", "iETH", "0xc383..." for v1; "v2", "iETHv2", "0xa0d3..." for v2
/// Default (None) → v1
pub fn resolve_vault_address(vault_query: Option<&str>) -> (&'static str, &'static VaultInfo) {
    match vault_query {
        None | Some("v1") | Some("iETH") | Some("ieth") => {
            (IETH_V1_VAULT, &VAULTS[0])
        }
        Some("v2") | Some("iETHv2") | Some("iethv2") => {
            (IETH_V2_VAULT, &VAULTS[1])
        }
        Some(addr) if addr.to_lowercase().starts_with("0xc383") => {
            (IETH_V1_VAULT, &VAULTS[0])
        }
        Some(addr) if addr.to_lowercase().starts_with("0xa0d3") => {
            (IETH_V2_VAULT, &VAULTS[1])
        }
        _ => (IETH_V1_VAULT, &VAULTS[0]), // default to v1
    }
}
