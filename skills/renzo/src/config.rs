/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// RestakeManager proxy contract (Renzo)
pub const RESTAKE_MANAGER: &str = "0x74a09653A083691711cF8215a6ab074BB4e99ef5";

/// ezETH token (liquid restaking token)
pub const EZETH_ADDRESS: &str = "0xbf5495Efe5DB9ce00f80364C8B423567e58d2110";

/// stETH token (Lido) — accepted as collateral
pub const STETH_ADDRESS: &str = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84";

/// RenzoOracle contract
#[allow(dead_code)]
pub const RENZO_ORACLE: &str = "0x5a12796f7e7ebbbc8a402667d266d2e65a814042";

/// Renzo REST API base URL (accessible in sandbox)
pub const API_BASE_URL: &str = "https://app.renzoprotocol.com/api";

/// Ethereum mainnet public RPC
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

// ----- Function selectors (verified with `cast sig`) -----

// RestakeManager
/// depositETH() → 0xf6326fb3
pub const SEL_DEPOSIT_ETH: &str = "f6326fb3";
/// deposit(address,uint256) → 0x47e7ef24
#[allow(dead_code)]
pub const SEL_DEPOSIT: &str = "47e7ef24";
/// calculateTVLs() → 0xff9969cd
pub const SEL_CALCULATE_TVLS: &str = "ff9969cd";
/// paused() → 0x5c975abb
pub const SEL_PAUSED: &str = "5c975abb";

// ERC-20 (ezETH / stETH)
/// balanceOf(address) → 0x70a08231
pub const SEL_BALANCE_OF: &str = "70a08231";
/// totalSupply() → 0x18160ddd
pub const SEL_TOTAL_SUPPLY: &str = "18160ddd";
/// approve(address,uint256) → 0x095ea7b3
#[allow(dead_code)]
pub const SEL_APPROVE: &str = "095ea7b3";
/// allowance(address,address) → 0xdd62ed3e
#[allow(dead_code)]
pub const SEL_ALLOWANCE: &str = "dd62ed3e";
