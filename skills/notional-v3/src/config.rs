// Notional Exponent (V4) configuration
// Protocol is deployed on Ethereum mainnet only

pub const ETHEREUM_CHAIN_ID: u64 = 1;
pub const ETHEREUM_RPC: &str = "https://ethereum.publicnode.com";

// MorphoLendingRouter on Ethereum mainnet
pub const MORPHO_LENDING_ROUTER: &str = "0x9a0c630C310030C4602d1A76583a3b16972ecAa0";

// AddressRegistry
pub const ADDRESS_REGISTRY: &str = "0xe335d314BD4eF7DD44F103dC124FEFb7Ce63eC95";

// Notional Exponent subgraph (no API key required for studio endpoint)
pub const SUBGRAPH_URL: &str =
    "https://api.studio.thegraph.com/query/60626/notional-exponent/version/latest";

// Underlying token addresses on Ethereum mainnet
pub const USDC_ETH: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const WETH_ETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";

// Known vault addresses on Ethereum mainnet
pub const VAULT_SUSDE: &str = "0xaf14d06a65c91541a5b2db627ecd1c92d7d9c48b";
pub const VAULT_MAPOLLO: &str = "0x091356e6793a0d960174eaab4d470e39a99dd673";
pub const VAULT_MHYPER: &str = "0x2a5c94fe8fa6c0c8d2a87e5c71ad628caa092ce4";
pub const VAULT_WEETH: &str = "0x7f723fee1e65a7d26be51a05af0b5efee4a7d5ae";
pub const VAULT_PT_SUSDE: &str = "0x0e61e810f0918081cbfd2ac8c97e5866daf3f622";
pub const VAULT_LIUSD: &str = "0x9fb57943926749b49a644f237a28b491c9b465e0";
pub const VAULT_OETH: &str = "0x2716561755154eef59bc48eb13712510b27f167f";
pub const VAULT_MHYPER2: &str = "0x94f6cb4fae0eb3fa74e9847dff2ff52fd5ec7e6e";

// Function selectors (verified with cast sig)
// healthFactor(address,address) = 0x576f5c40
pub const SEL_HEALTH_FACTOR: &str = "0x576f5c40";
// balanceOfCollateral(address,address) = 0xda3a855f
pub const SEL_BALANCE_OF_COLLATERAL: &str = "0xda3a855f";
// enterPosition(address,address,uint256,uint256,bytes) = 0xde13c617
pub const SEL_ENTER_POSITION: &str = "0xde13c617";
// exitPosition(address,address,uint256,uint16,bytes) = 0x8a363181
pub const SEL_EXIT_POSITION: &str = "0x8a363181";
// initiateWithdraw(address,address,uint256) = 0x37753799
pub const SEL_INITIATE_WITHDRAW: &str = "0x37753799";
// claimRewards(address,address) = 0xf1e42ccd
pub const SEL_CLAIM_REWARDS: &str = "0xf1e42ccd";
// approve(address,uint256) = 0x095ea7b3
pub const SEL_APPROVE: &str = "0x095ea7b3";
