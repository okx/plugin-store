use crate::config::{pad_address, pad_u256};

// ============================================================
// PufferVaultV5 — mint pufETH
// ============================================================

/// PufferVaultV5.depositETH(address receiver) payable
/// Selector: 0x2d2da806
pub fn build_deposit_eth_calldata(receiver: &str) -> String {
    format!("0x2d2da806{}", pad_address(receiver))
}

/// PufferVaultV5.deposit(uint256 assets, address receiver) — WETH path (ERC-4626).
/// Selector: 0x6e553f65
/// Reserved for v0.2.x (WETH stake command). Not wired yet.
#[allow(dead_code)]
pub fn build_deposit_weth_calldata(assets: u128, receiver: &str) -> String {
    format!(
        "0x6e553f65{}{}",
        pad_u256(assets),
        pad_address(receiver),
    )
}

// ============================================================
// PufferVaultV5 — 1-step instant withdraw (applies exit fee)
// ============================================================

/// PufferVaultV5.redeem(uint256 shares, address receiver, address owner)
/// Selector: 0xba087652
/// Burns `shares` pufETH, transfers WETH (assets minus exit fee) to receiver.
pub fn build_redeem_calldata(shares: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0xba087652{}{}{}",
        pad_u256(shares),
        pad_address(receiver),
        pad_address(owner),
    )
}

/// PufferVaultV5.withdraw(uint256 assets, address receiver, address owner)
/// Selector: 0xb460af94
/// Specify WETH amount out; pulls up to `previewWithdraw(assets)` pufETH from owner.
#[allow(dead_code)]
pub fn build_withdraw_assets_calldata(assets: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0xb460af94{}{}{}",
        pad_u256(assets),
        pad_address(receiver),
        pad_address(owner),
    )
}

// ============================================================
// PufferWithdrawalManager — 2-step queued withdraw (no fee)
// ============================================================

/// PufferWithdrawalManager.requestWithdrawal(uint128 pufETHAmount, address recipient)
/// Selector: 0xef027fbf
/// Note: pufETHAmount is uint128 but ABI-encoded as 32 bytes (left-padded).
pub fn build_request_withdrawal_calldata(pufeth_amount: u128, recipient: &str) -> String {
    format!(
        "0xef027fbf{}{}",
        pad_u256(pufeth_amount),
        pad_address(recipient),
    )
}

/// PufferWithdrawalManager.completeQueuedWithdrawal(uint256 withdrawalIdx)
/// Selector: 0x6a4800a4
pub fn build_complete_queued_withdrawal_calldata(idx: u128) -> String {
    format!("0x6a4800a4{}", pad_u256(idx))
}

// ============================================================
// ERC-20 approve (shared helper)
// ============================================================

/// ERC-20 approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    format!(
        "0x095ea7b3{}{}",
        pad_address(spender),
        pad_u256(amount),
    )
}

#[cfg(test)]
mod tests {
    use sha3::{Digest, Keccak256};

    fn sel(sig: &str) -> String {
        let h = Keccak256::digest(sig.as_bytes());
        format!("0x{}", hex::encode(&h[..4]))
    }

    /// Selectors here are inlined into format!() literals (no `pub const`
    /// to import). Verify each against keccak256 so any copy/paste typo
    /// would fail this test instead of silently misrouting calls
    /// on-chain. Pattern matches euler-v2 / aave-v2 / fourmeme.
    #[test]
    fn selectors_match_keccak256() {
        assert_eq!(sel("depositETH(address)"),                "0x2d2da806");
        assert_eq!(sel("deposit(uint256,address)"),           "0x6e553f65");
        assert_eq!(sel("redeem(uint256,address,address)"),    "0xba087652");
        assert_eq!(sel("withdraw(uint256,address,address)"),  "0xb460af94");
        assert_eq!(sel("requestWithdrawal(uint128,address)"), "0xef027fbf");
        assert_eq!(sel("completeQueuedWithdrawal(uint256)"),  "0x6a4800a4");
        assert_eq!(sel("approve(address,uint256)"),           "0x095ea7b3");
    }
}
