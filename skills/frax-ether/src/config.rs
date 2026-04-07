/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// Ethereum mainnet RPC URL
pub const ETH_RPC_URL: &str = "https://ethereum.publicnode.com";

/// frxETHMinter contract — deposit ETH to receive frxETH
pub const FRXETH_MINTER: &str = "0xbAFA44EFE7901E04E39Dad13167D089C559c1138";

/// frxETH ERC-20 token
pub const FRXETH_TOKEN: &str = "0x5E8422345238F34275888049021821E8E08CAa1f";

/// sfrxETH ERC-4626 vault — deposit frxETH to earn yield
pub const SFRXETH_VAULT: &str = "0xac3E018457B222d93114458476f3E3416Abbe38F";

/// Frax Finance API base URL
pub const FRAX_API_URL: &str = "https://api.frax.finance";

// === Function Selectors (verified with `cast sig`) ===

/// submit() — frxETHMinter: deposit ETH, receive frxETH (no args)
/// cast sig "submit()" → 0x5bcb2fc6 ✅
/// Note: submit(address) (0xa1903eab) reverts with our wallet; submit() works correctly.
pub const SEL_SUBMIT: &str = "5bcb2fc6";

/// deposit(uint256,address) — ERC-4626 deposit
/// cast sig "deposit(uint256,address)" → 0x6e553f65 ✅
pub const SEL_DEPOSIT: &str = "6e553f65";

/// redeem(uint256,address,address) — ERC-4626 redeem
/// cast sig "redeem(uint256,address,address)" → 0xba087652 ✅
pub const SEL_REDEEM: &str = "ba087652";

/// convertToAssets(uint256) — ERC-4626 price query
/// cast sig "convertToAssets(uint256)" → 0x07a2d13a ✅
pub const SEL_CONVERT_TO_ASSETS: &str = "07a2d13a";

/// convertToShares(uint256) — ERC-4626 reverse price query
/// cast sig "convertToShares(uint256)" → 0xc6e6f592 ✅
pub const SEL_CONVERT_TO_SHARES: &str = "c6e6f592";

/// balanceOf(address) — ERC-20 balance
/// cast sig "balanceOf(address)" → 0x70a08231 ✅
pub const SEL_BALANCE_OF: &str = "70a08231";

/// totalAssets() — ERC-4626 total assets
/// cast sig "totalAssets()" → 0x01e1d114 ✅
pub const SEL_TOTAL_ASSETS: &str = "01e1d114";

/// approve(address,uint256) — ERC-20 approve
/// cast sig "approve(address,uint256)" → 0x095ea7b3 ✅
pub const SEL_APPROVE: &str = "095ea7b3";
