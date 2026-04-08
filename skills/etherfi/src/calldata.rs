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

/// Build calldata for weETH.redeem(uint256 shares, address receiver, address owner)
/// This is the ERC-4626 redeem: unwraps weETH → eETH.
/// Selector: 0xba087652 (keccak256("redeem(uint256,address,address)")[0..4])
///
/// ABI layout:
///   [0..4]    selector 0xba087652
///   [4..36]   shares (uint256 = weETH amount in wei)
///   [36..68]  receiver (address, padded to 32 bytes)
///   [68..100] owner (address, padded to 32 bytes — same as receiver for self-redeem)
pub fn build_unwrap_calldata(shares: u128, receiver: &str) -> String {
    format!(
        "0xba087652{}{}{}",
        pad_u256(shares),
        pad_address(receiver),
        pad_address(receiver),
    )
}
