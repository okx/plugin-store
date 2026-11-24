/// Fluid Protocol contract addresses — same on Ethereum (1) and Arbitrum (42161).
pub const VAULT_RESOLVER: &str      = "0xA5C3E16523eeeDDcC34706b0E6bE88b4c6EA95cC";
pub const POSITIONS_RESOLVER: &str  = "0xaA21a86030EAa16546A759d2d10fd3bF9D053Bc7";
pub const NFT_CONTRACT: &str        = "0x324c5dc1fc42c7a4d43d92df1eba58a54d13bf2d";

/// Native ETH sentinel address used by Fluid (and many DeFi protocols).
pub const NATIVE_ETH: &str = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

/// VaultEntireData struct layout (from on-chain reverse engineering):
/// Each vault entry from getVaultEntireData/getVaultsEntireData is exactly 97 words (3104 bytes).
pub const VAULT_STRUCT_WORDS: usize = 97;
/// Word index for isSmartCol (bool)
pub const WORD_IS_SMART_COL: usize = 1;
/// Word index for isSmartDebt (bool)
pub const WORD_IS_SMART_DEBT: usize = 2;
/// Word index for NFT contract address
pub const WORD_NFT_CONTRACT: usize = 4;
/// Word index for collateral token address
pub const WORD_COL_TOKEN: usize = 11;
/// Word index for debt token address
pub const WORD_DEBT_TOKEN: usize = 13;
/// Word index for borrowRateVault within ExchangePricesAndRates.
/// Layout: words 0-2 (vault/flags) + 18 (ConstantViews) + 13 (Configs, each uint16 gets own slot) = 33;
/// ExchangePricesAndRates starts at word 34; borrowRateVault is index 11 within it → word 45.
/// Rate is stored in 1e2 precision: 1% = 100. Divide by 100.0 to display as a percentage.
pub const WORD_BORROW_RATE_VAULT: usize = 45;
