use crate::config::{pad_u256, pad_address};

/// Build calldata for LiquidityPool.deposit()
/// Selector: 0xd0e30db0 (keccak256("deposit()")[0..4])
/// ETH value is passed as the native msg.value — no ABI arguments.
/// The ether.fi LiquidityPool accepts plain deposit() with no referral param.
pub fn build_deposit_calldata() -> String {
    "0xd0e30db0".to_string()
}

/// Build calldata for weETH.wrap(uint256 _eETHAmount)
/// Wraps eETH → weETH on the ether.fi weETH contract.
/// Selector: 0xea598cb0 (keccak256("wrap(uint256)")[0..4])
///
/// ABI layout:
///   [0..4]   selector 0xea598cb0
///   [4..36]  _eETHAmount (uint256 = eETH amount in wei)
pub fn build_wrap_calldata(assets: u128, _receiver: &str) -> String {
    format!("0xea598cb0{}", pad_u256(assets))
}

/// Build calldata for weETH.unwrap(uint256 _weETHAmount)
/// Unwraps weETH → eETH on the ether.fi weETH contract.
/// Selector: 0xde0e9a3e (keccak256("unwrap(uint256)")[0..4])
///
/// Note: weETH does NOT implement ERC-4626 redeem(uint256,address,address).
/// The contract only exposes wrap(uint256) and unwrap(uint256).
///
/// ABI layout:
///   [0..4]   selector 0xde0e9a3e
///   [4..36]  _weETHAmount (uint256 = weETH amount in wei)
pub fn build_unwrap_calldata(shares: u128, _receiver: &str) -> String {
    format!("0xde0e9a3e{}", pad_u256(shares))
}

/// Build calldata for LiquidityPool.requestWithdraw(address recipient, uint256 amountOfEEth)
/// Selector: 0x397a1b28 (keccak256("requestWithdraw(address,uint256)")[0..4])
///
/// Burns the caller's eETH (via ERC-20 transferFrom) and mints a WithdrawRequestNFT.
/// Caller must approve LiquidityPool to spend eETH before calling this.
///
/// ABI layout:
///   [0..4]    selector 0x397a1b28
///   [4..36]   recipient (address, padded to 32 bytes)
///   [36..68]  amountOfEEth (uint256 = eETH amount in wei)
pub fn build_request_withdraw_calldata(recipient: &str, amount_wei: u128) -> String {
    format!(
        "0x397a1b28{}{}",
        pad_address(recipient),
        pad_u256(amount_wei),
    )
}

/// Build calldata for WithdrawRequestNFT.claimWithdraw(uint256 tokenId)
/// Selector: 0xb13acedd (keccak256("claimWithdraw(uint256)")[0..4])
///
/// Burns the WithdrawRequestNFT and sends ETH to the recipient.
/// Only callable after the withdrawal request has been finalized.
///
/// ABI layout:
///   [0..4]    selector 0xb13acedd
///   [4..36]   tokenId (uint256)
pub fn build_claim_withdraw_calldata(token_id: u64) -> String {
    format!("0xb13acedd{:0>64x}", token_id)
}
