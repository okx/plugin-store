/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// USDe token (Ethena synthetic dollar)
pub const USDE_ADDRESS: &str = "0x4c9EDD5852cd905f086C759E8383e09bff1E68B3";

/// sUSDe ERC-4626 vault (staked USDe)
pub const SUSDE_ADDRESS: &str = "0x9D39A5DE30e57443BfF2A8307A4256c8797A3497";

/// Ethereum mainnet RPC (publicnode avoids rate limits)
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

/// Ethena yield API endpoint
pub const YIELD_API_URL: &str = "https://app.ethena.fi/api/yields/protocol-and-staking-yield";

/// Token decimals (both USDe and sUSDe use 18 decimals)
pub const TOKEN_DECIMALS: u32 = 18;

// Function selectors (verified with cast sig)

/// approve(address,uint256) = 0x095ea7b3
pub const SEL_APPROVE: &str = "095ea7b3";

/// deposit(uint256,address) = 0x6e553f65 (ERC-4626)
pub const SEL_DEPOSIT: &str = "6e553f65";

/// redeem(uint256,address,address) = 0xba087652 (ERC-4626 standard redeem — not used for Ethena, Ethena uses cooldown)
#[allow(dead_code)]
pub const SEL_REDEEM: &str = "ba087652";

/// cooldownShares(uint256) = 0x9343d9e1
pub const SEL_COOLDOWN_SHARES: &str = "9343d9e1";

/// cooldownAssets(uint256) = 0xcdac52ed
pub const SEL_COOLDOWN_ASSETS: &str = "cdac52ed";

/// unstake(address) = 0xf2888dbb
pub const SEL_UNSTAKE: &str = "f2888dbb";

/// balanceOf(address) = 0x70a08231
pub const SEL_BALANCE_OF: &str = "70a08231";

/// convertToAssets(uint256) = 0x07a2d13a
pub const SEL_CONVERT_TO_ASSETS: &str = "07a2d13a";

/// convertToShares(uint256) = 0xc6e6f592
pub const SEL_CONVERT_TO_SHARES: &str = "c6e6f592";

/// previewDeposit(uint256) = 0xef8b30f7
pub const SEL_PREVIEW_DEPOSIT: &str = "ef8b30f7";

/// previewRedeem(uint256) = 0x4cdad506
pub const SEL_PREVIEW_REDEEM: &str = "4cdad506";

/// cooldowns(address) = 0x01320fe2 (verified with cast sig)
pub const SEL_COOLDOWNS: &str = "01320fe2";

/// cooldownDuration() = 0x35269315
pub const SEL_COOLDOWN_DURATION: &str = "35269315";

/// totalAssets() = 0x01e1d114
pub const SEL_TOTAL_ASSETS: &str = "01e1d114";
