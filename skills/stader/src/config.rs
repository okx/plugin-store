// Stader ETHx Liquid Staking — Contract Addresses & Constants
// Chain: Ethereum Mainnet (chain ID 1)

#[allow(dead_code)]
pub const CHAIN_ID: u64 = 1;

// Contract addresses — Ethereum Mainnet
pub const STADER_MANAGER: &str = "0xcf5EA1b38380f6aF39068375516Daf40Ed70D299";
pub const USER_WITHDRAW_MANAGER: &str = "0x9F0491B32DBce587c50c4C43AB303b06478193A7";
pub const ETHX_TOKEN: &str = "0xA35b1B31Ce002FBF2058D22F30f95D405200A15b";

// RPC endpoint — ethereum.publicnode.com (no rate limit issues per kb)
#[allow(dead_code)]
pub const ETH_RPC_URL: &str = "https://ethereum.publicnode.com";

// Protocol constants
pub const MIN_DEPOSIT_WEI: u64 = 100_000_000_000_000; // 0.0001 ETH — protocol enforced minimum
