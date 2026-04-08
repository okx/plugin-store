/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// stETH proxy contract (Lido)
pub const STETH_ADDRESS: &str = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84";

/// wstETH contract
#[allow(dead_code)]
pub const WSTETH_ADDRESS: &str = "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0";

/// WithdrawalQueueERC721 proxy
pub const WITHDRAWAL_QUEUE_ADDRESS: &str = "0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1";

/// Lido REST API base URL
pub const API_BASE_URL: &str = "https://eth-api.lido.fi";

/// Withdrawal queue REST API base URL
pub const WQ_API_BASE_URL: &str = "https://wq-api.lido.fi";

/// Min withdrawal amount in wei (protocol enforced)
pub const MIN_WITHDRAWAL_WEI: u128 = 100;

/// Max withdrawal amount in wei: 1000 ETH
pub const MAX_WITHDRAWAL_WEI: u128 = 1_000_000_000_000_000_000_000;

// Function selectors — stETH
pub const SEL_SUBMIT: &str = "a1903eab";
pub const SEL_BALANCE_OF: &str = "70a08231";
pub const SEL_SHARES_OF: &str = "f5eb42dc";
pub const SEL_IS_STAKING_PAUSED: &str = "1ea7ca89";

// Function selectors — WithdrawalQueueERC721
pub const SEL_GET_LAST_CHECKPOINT_INDEX: &str = "526eae3e";
