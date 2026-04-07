// Chain and contract configuration for Yearn Finance

pub const ETHEREUM_CHAIN_ID: u64 = 1;
pub const ETHEREUM_RPC: &str = "https://ethereum.publicnode.com";
pub const YDAEMON_BASE_URL: &str = "https://ydaemon.yearn.fi";

// Known vault addresses (for testing; all vaults resolved dynamically via yDaemon API)
pub const YVUSDT1_VAULT: &str = "0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa";

// ERC-20 token addresses on Ethereum mainnet
pub const USDT_ADDR: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
pub const USDC_ADDR: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const DAI_ADDR: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
pub const WETH_ADDR: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

// Function selectors (all verified via `cast sig`)
pub mod selectors {
    /// deposit(uint256,address) — ERC-4626
    pub const DEPOSIT: &str = "0x6e553f65";
    /// redeem(uint256,address,address) — ERC-4626
    pub const REDEEM: &str = "0xba087652";
    /// approve(address,uint256) — ERC-20
    pub const APPROVE: &str = "0x095ea7b3";
    /// balanceOf(address) — ERC-20/ERC-4626
    pub const BALANCE_OF: &str = "0x70a08231";
    /// pricePerShare() — Yearn vault
    pub const PRICE_PER_SHARE: &str = "0x99530b06";
    /// totalAssets() — ERC-4626
    pub const TOTAL_ASSETS: &str = "0x01e1d114";
    /// asset() — ERC-4626
    pub const ASSET: &str = "0x38d52e0f";
}
