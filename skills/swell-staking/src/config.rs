/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// Ethereum RPC URL
pub const ETH_RPC_URL: &str = "https://ethereum.publicnode.com";

/// swETH proxy contract (Swell liquid staking)
pub const SWETH_ADDRESS: &str = "0xf951E335afb289353dc249e82926178EaC7DEd78";

/// rswETH proxy contract (Swell liquid restaking via EigenLayer)
pub const RSWETH_ADDRESS: &str = "0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0";

// ─── Function selectors — verified via `cast sig` ───────────────────────────

/// deposit() — payable, no parameters. Used for both swETH and rswETH.
/// cast sig "deposit()" = 0xd0e30db0
pub const SEL_DEPOSIT: &str = "d0e30db0";

/// depositWithReferral(address) — payable, referral address.
/// cast sig "depositWithReferral(address)" = 0xc18d7cb7
pub const SEL_DEPOSIT_WITH_REFERRAL: &str = "c18d7cb7";

/// swETHToETHRate() — returns uint256 (1 swETH in ETH, 18 decimals)
/// cast sig "swETHToETHRate()" = 0xd68b2cb6
pub const SEL_SWETH_TO_ETH_RATE: &str = "d68b2cb6";

/// ethToSwETHRate() — returns uint256 (1 ETH in swETH, 18 decimals)
/// cast sig "ethToSwETHRate()" = 0x0de3ff57
pub const SEL_ETH_TO_SWETH_RATE: &str = "0de3ff57";

/// rswETHToETHRate() — returns uint256 (1 rswETH in ETH, 18 decimals)
/// cast sig "rswETHToETHRate()" = 0xa7b9544e
pub const SEL_RSWETH_TO_ETH_RATE: &str = "a7b9544e";

/// ethToRswETHRate() — returns uint256 (1 ETH in rswETH, 18 decimals)
/// cast sig "ethToRswETHRate()" = 0x780a47e0
pub const SEL_ETH_TO_RSWETH_RATE: &str = "780a47e0";

/// balanceOf(address) — ERC20 standard
/// cast sig "balanceOf(address)" = 0x70a08231
pub const SEL_BALANCE_OF: &str = "70a08231";

/// totalSupply() — ERC20 standard
/// cast sig "totalSupply()" = 0x18160ddd
pub const SEL_TOTAL_SUPPLY: &str = "18160ddd";
