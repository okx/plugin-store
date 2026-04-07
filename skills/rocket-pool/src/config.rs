/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// RocketStorage — permanent registry contract (never changes)
pub const ROCKET_STORAGE: &str = "0x1d8f8f00cfa6758d7bE78336684788Fb0ee0Fa46";

/// Ethereum public RPC endpoint
#[allow(dead_code)]
pub const ETH_RPC: &str = "https://ethereum.publicnode.com";

/// Minimum deposit amount enforced by Rocket Pool protocol (0.01 ETH in wei)
pub const MIN_DEPOSIT_WEI: u128 = 10_000_000_000_000_000; // 0.01 ETH

// ── RocketStorage key hashes (keccak256 of contract name strings) ─────────────
// These are used in getAddress(bytes32) calls to resolve current contract addresses.

/// keccak256("contract.addressrocketDepositPool")
pub const KEY_DEPOSIT_POOL: &str =
    "65dd923ddfc8d8ae6088f80077201d2403cbd565f0ba25e09841e2799ec90bb2";

/// keccak256("contract.addressrocketTokenRETH")
pub const KEY_TOKEN_RETH: &str =
    "e3744443225bff7cc22028be036b80de58057d65a3fdca0a3df329f525e31ccc";

/// keccak256("contract.addressrocketNetworkBalances")
pub const KEY_NETWORK_BALANCES: &str =
    "7630e125f1c009e5fc974f6dae77c6d5b1802979b36e6d7145463c21782af01e";

/// keccak256("contract.addressrocketNodeManager")
pub const KEY_NODE_MANAGER: &str =
    "af00be55c9fb8f543c04e0aa0d70351b880c1bfafffd15b60065a4a50c85ec94";

/// keccak256("contract.addressrocketMinipoolManager")
pub const KEY_MINIPOOL_MANAGER: &str =
    "e9dfec9339b94a131861a58f1bb4ac4c1ce55c7ffe8550e0b6ebcfde87bb012f";

// ── Function selectors ────────────────────────────────────────────────────────

/// getAddress(bytes32) on RocketStorage
pub const SEL_GET_ADDRESS: &str = "21f8a721";

/// deposit() on RocketDepositPool — payable
pub const SEL_DEPOSIT: &str = "d0e30db0";

/// burn(uint256) on RocketTokenRETH
pub const SEL_BURN: &str = "42966c68";

/// getExchangeRate() on RocketTokenRETH
pub const SEL_GET_EXCHANGE_RATE: &str = "e6aa216c";

/// balanceOf(address) on rETH ERC20
pub const SEL_BALANCE_OF: &str = "70a08231";

/// totalSupply() on rETH ERC20
pub const SEL_TOTAL_SUPPLY: &str = "18160ddd";

/// getBalance() on RocketDepositPool
pub const SEL_GET_DEPOSIT_BALANCE: &str = "12065fe0";

/// getTotalETHBalance() on RocketNetworkBalances
pub const SEL_GET_TOTAL_ETH: &str = "964d042c";

/// getTotalRETHSupply() on RocketNetworkBalances
#[allow(dead_code)]
pub const SEL_GET_TOTAL_RETH: &str = "c4c8d0ad";

/// getNodeCount() on RocketNodeManager
pub const SEL_GET_NODE_COUNT: &str = "39bf397e";

/// getMinipoolCount() on RocketMinipoolManager
pub const SEL_GET_MINIPOOL_COUNT: &str = "ae4d0bed";

/// Rocket Pool APR API endpoint
pub const ROCKETPOOL_APR_API: &str = "https://api.rocketpool.net/api/apr";
