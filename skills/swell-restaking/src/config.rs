/// rswETH proxy contract (Swell liquid restaking via EigenLayer)
/// Address verified: https://etherscan.io/address/0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0
pub const RSWETH_ADDRESS: &str = "0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0";

// ─── Function selectors — verified via `cast sig` ───────────────────────────

/// deposit() — payable, no parameters. ETH in → rswETH out.
/// cast sig "deposit()" = 0xd0e30db0  ✅
pub const SEL_DEPOSIT: &str = "d0e30db0";

/// rswETHToETHRate() — returns uint256 (1 rswETH in ETH, 18 decimals)
/// cast sig "rswETHToETHRate()" = 0xa7b9544e  ✅
pub const SEL_RSWETH_TO_ETH_RATE: &str = "a7b9544e";

/// ethToRswETHRate() — returns uint256 (1 ETH in rswETH, 18 decimals)
/// cast sig "ethToRswETHRate()" = 0x780a47e0  ✅
pub const SEL_ETH_TO_RSWETH_RATE: &str = "780a47e0";

/// totalETHDeposited() — returns uint256 total ETH deposited into rswETH pool
/// cast sig "totalETHDeposited()" = 0x7b2c9070  ✅
pub const SEL_TOTAL_ETH_DEPOSITED: &str = "7b2c9070";

/// balanceOf(address) — ERC20 standard balance query
/// cast sig "balanceOf(address)" = 0x70a08231  ✅
pub const SEL_BALANCE_OF: &str = "70a08231";

/// totalSupply() — ERC20 standard total supply
/// cast sig "totalSupply()" = 0x18160ddd  ✅
pub const SEL_TOTAL_SUPPLY: &str = "18160ddd";
