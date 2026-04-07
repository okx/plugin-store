/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// rsETH token proxy address
pub const RSETH_ADDRESS: &str = "0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7";

/// LRTDepositPool proxy address
pub const DEPOSIT_POOL_ADDRESS: &str = "0x036676389e48133B63a802f8635AD39E752D375D";

/// LRTOracle proxy address
pub const ORACLE_ADDRESS: &str = "0x349A73444b1a310BAe67ef67973022020d70020d";

/// LRTWithdrawalManager proxy address
pub const WITHDRAWAL_MANAGER_ADDRESS: &str = "0x62De59c08eB5dAE4b7E6F7a8cAd3006d6965ec16";

/// Sentinel address used by Kelp for ETH (not a real ERC-20)
pub const ETH_ASSET_ADDRESS: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

// Function selectors (Keccak-256 of signature, first 4 bytes)

/// depositETH(uint256,string) — payable, LRTDepositPool
pub const SEL_DEPOSIT_ETH: &str = "72c51c0b";

/// depositAsset(address,uint256,uint256,string) — LRTDepositPool
#[allow(dead_code)]
pub const SEL_DEPOSIT_ASSET: &str = "c3ae1766";

/// getRsETHAmountToMint(address,uint256) — LRTDepositPool view
pub const SEL_GET_RSETH_AMOUNT: &str = "ba5bb442";

/// rsETHPrice() — LRTOracle view, returns price in 1e18 units (ETH per rsETH)
pub const SEL_RSETH_PRICE: &str = "b4b46434";

/// getAssetCurrentPrice(address) — LRTOracle view
#[allow(dead_code)]
pub const SEL_ASSET_PRICE: &str = "7a95e516";

/// balanceOf(address) — ERC-20
pub const SEL_BALANCE_OF: &str = "70a08231";

/// totalSupply() — ERC-20
#[allow(dead_code)]
pub const SEL_TOTAL_SUPPLY: &str = "18160ddd";

/// initiateWithdrawal(address,uint256) — LRTWithdrawalManager
pub const SEL_INITIATE_WITHDRAWAL: &str = "c8393ba9";

/// completeWithdrawal(address) — LRTWithdrawalManager
#[allow(dead_code)]
pub const SEL_COMPLETE_WITHDRAWAL: &str = "6dbaf9ee";

/// CoinGecko API for rsETH price/APY data
pub const COINGECKO_API: &str =
    "https://api.coingecko.com/api/v3/simple/price?ids=kelp-dao-restaked-eth&vs_currencies=eth,usd&include_24hr_change=true";

/// Kelp DAO rsETH APY endpoint (community/unofficial)
pub const KELP_APY_API: &str = "https://api.coingecko.com/api/v3/coins/kelp-dao-restaked-eth";
