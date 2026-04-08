use crate::config::{pad_address, pad_u256};

/// Build calldata for LiquidityPool.deposit(address _referral)
/// Selector: 0x5340a0d5 (keccak256("deposit(address)")[0..4])
/// ETH value is passed as the native msg.value, not ABI-encoded.
/// The _referral address is set to the zero address (no referral).
///
/// ABI layout:
///   [0..4]   selector 0x5340a0d5
///   [4..36]  _referral (address, padded to 32 bytes)
pub fn build_deposit_calldata() -> String {
    // No referral: pass zero address
    let referral = pad_address("0x0000000000000000000000000000000000000000");
    format!("0x5340a0d5{}", referral)
}

/// Build calldata for weETH.deposit(uint256 assets, address receiver)
/// This is the ERC-4626 deposit: wraps eETH → weETH.
/// Selector: 0x6e553f65 (keccak256("deposit(uint256,address)")[0..4])
///
/// ABI layout:
///   [0..4]    selector 0x6e553f65
///   [4..36]   assets (uint256 = eETH amount in wei)
///   [36..68]  receiver (address, padded to 32 bytes)
pub fn build_wrap_calldata(assets: u128, receiver: &str) -> String {
    format!(
        "0x6e553f65{}{}",
        pad_u256(assets),
        pad_address(receiver),
    )
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
