/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// StoneVault contract (Ethereum mainnet)
pub const STONE_VAULT: &str = "0xA62F9C5af106FeEE069F38dE51098D9d81B90572";

/// STONE token contract (Ethereum mainnet)
pub const STONE_TOKEN: &str = "0x7122985656e38BDC0302Db86685bb972b145bD3C";

/// Ethereum public RPC
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

// Function selectors — verified with `cast sig`
/// deposit()  → 0xd0e30db0
pub const SEL_DEPOSIT: &str = "d0e30db0";
/// requestWithdraw(uint256) → 0x745400c9
pub const SEL_REQUEST_WITHDRAW: &str = "745400c9";
/// cancelWithdraw(uint256) → 0x9f01f7ba
pub const SEL_CANCEL_WITHDRAW: &str = "9f01f7ba";
/// currentSharePrice() → 0x28a79576
pub const SEL_CURRENT_SHARE_PRICE: &str = "28a79576";
/// latestRoundID() → 0xf76339dc
pub const SEL_LATEST_ROUND_ID: &str = "f76339dc";
/// withdrawFeeRate() → 0xea99e689
pub const SEL_WITHDRAW_FEE_RATE: &str = "ea99e689";
/// userReceipts(address) → 0xa4786f3d
pub const SEL_USER_RECEIPTS: &str = "a4786f3d";
/// getVaultAvailableAmount() → 0x82f1631f
pub const SEL_GET_VAULT_AVAILABLE: &str = "82f1631f";
/// balanceOf(address) → 0x70a08231
pub const SEL_BALANCE_OF: &str = "70a08231";
