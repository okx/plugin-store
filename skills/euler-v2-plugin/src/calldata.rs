//! ABI-encoded calldata builders for EVC, EVK, and ERC-20 calls.
//!
//! All function selectors are computed at compile time as constants. Calldata is
//! returned as `String` with `0x` prefix (ready to feed into onchainos --input-data).
//!
//! Several helpers are kept unused in v0.1 (build_approve, build_deposit, build_repay,
//! SEL_GET_COLLATERALS/CONTROLLERS, etc.) — they're the standard ERC-4626 / EVC
//! counterparts that the plugin avoids today because OKX TEE rejects them for
//! un-whitelisted vaults (see ONC-001). Once OKX whitelists Euler v2, the plugin can
//! switch to these single-tx paths instead of the donate+skim / repayWithShares
//! two-tx workarounds.

#![allow(dead_code)]

use crate::rpc::pad_address;

/// Pad a u128 amount as a 64-char hex uint256 (big-endian).
fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode `uint256::max` as 64 'f' chars (used by LEND-001 "repay all" pattern).
const MAX_UINT256_HEX: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

// ─── ERC-20 ────────────────────────────────────────────────────────────────────

/// `approve(address spender, uint256 amount)` selector = 0x095ea7b3.
const SEL_APPROVE: &str = "095ea7b3";

pub fn build_approve(spender: &str, amount: u128) -> String {
    format!("0x{}{}{}", SEL_APPROVE, pad_address(spender), pad_u128(amount))
}

pub fn build_approve_max(spender: &str) -> String {
    format!("0x{}{}{}", SEL_APPROVE, pad_address(spender), MAX_UINT256_HEX)
}

// ─── ERC-4626 (EVK vault) ──────────────────────────────────────────────────────

/// `deposit(uint256 assets, address receiver)` selector = 0x6e553f65.
const SEL_DEPOSIT: &str = "6e553f65";

pub fn build_deposit(assets: u128, receiver: &str) -> String {
    format!("0x{}{}{}", SEL_DEPOSIT, pad_u128(assets), pad_address(receiver))
}

/// `withdraw(uint256 assets, address receiver, address owner)` selector = 0xb460af94.
const SEL_WITHDRAW: &str = "b460af94";

pub fn build_withdraw(assets: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0x{}{}{}{}",
        SEL_WITHDRAW, pad_u128(assets), pad_address(receiver), pad_address(owner)
    )
}

/// `redeem(uint256 shares, address receiver, address owner)` selector = 0xba087652.
/// Used for "withdraw all" since redeeming the user's full share count avoids
/// rounding-down dust that `withdraw(assets, ...)` could leave behind.
const SEL_REDEEM: &str = "ba087652";

pub fn build_redeem(shares: u128, receiver: &str, owner: &str) -> String {
    format!(
        "0x{}{}{}{}",
        SEL_REDEEM, pad_u128(shares), pad_address(receiver), pad_address(owner)
    )
}

/// EVK `skim(uint256 amount, address receiver)` — credit the vault's excess
/// underlying-asset balance to `receiver` as shares. Selector `0x8d56c639`.
///
/// The "donation+skim" pattern bypasses ERC-4626 `deposit`'s `transferFrom` call,
/// which OKX TEE wallet rejects for un-whitelisted vaults. Flow:
///   1. user calls `IERC20(asset).transfer(vault, amount)` (top-level on the
///      whitelisted asset contract — TEE accepts)
///   2. user calls `vault.skim(amount, user)` (no internal `transferFrom` —
///      TEE accepts; vault sees its own balance went up and mints shares)
const SEL_SKIM: &str = "8d56c639";

pub fn build_skim(amount: u128, receiver: &str) -> String {
    format!("0x{}{}{}", SEL_SKIM, pad_u128(amount), pad_address(receiver))
}

/// ERC-20 `transfer(address recipient, uint256 amount)` — used for the "donate"
/// step of the skim pattern. Selector 0xa9059cbb.
const SEL_TRANSFER: &str = "a9059cbb";

pub fn build_erc20_transfer(recipient: &str, amount: u128) -> String {
    format!("0x{}{}{}", SEL_TRANSFER, pad_address(recipient), pad_u128(amount))
}

// ─── EVK borrow / repay ────────────────────────────────────────────────────────

/// `borrow(uint256 amount, address receiver)` selector = 0x4b3fd148.
const SEL_BORROW: &str = "4b3fd148";

pub fn build_borrow(amount: u128, receiver: &str) -> String {
    format!("0x{}{}{}", SEL_BORROW, pad_u128(amount), pad_address(receiver))
}

/// `repay(uint256 amount, address receiver)` selector = 0xacb70815.
///
/// **NOTE**: Direct `repay` triggers `IERC20(asset).transferFrom(user, vault, amount)`
/// which OKX TEE wallet rejects for un-whitelisted vaults (see [ONC-001]).
/// Use `build_repay_with_shares` instead — same end-state, no transferFrom.
const SEL_REPAY: &str = "acb70815";

pub fn build_repay(amount: u128, receiver: &str) -> String {
    format!("0x{}{}{}", SEL_REPAY, pad_u128(amount), pad_address(receiver))
}

/// `repayWithShares(uint256 amount, address receiver)` selector = 0xa9c8eb7e.
///
/// Burns the caller's vault shares to reduce `receiver`'s debt by `amount`.
/// Used in place of `repay()` to bypass OKX TEE wallet's anti-drain check —
/// no `transferFrom` is invoked, so the call passes TEE policy.
///
/// Per LEND-001, calling with `amount = type(uint256).max` repays full debt
/// including just-accrued interest. Vault burns just enough shares.
const SEL_REPAY_WITH_SHARES: &str = "a9c8eb7e";

pub fn build_repay_with_shares(amount: u128, receiver: &str) -> String {
    format!("0x{}{}{}", SEL_REPAY_WITH_SHARES, pad_u128(amount), pad_address(receiver))
}

pub fn build_repay_with_shares_all(receiver: &str) -> String {
    format!("0x{}{}{}", SEL_REPAY_WITH_SHARES, MAX_UINT256_HEX, pad_address(receiver))
}

/// `disableController()` selector = 0x869e50c7. No args. Called on the controller
/// vault itself (not the EVC); EVK then notifies the EVC to clear the role.
const SEL_DISABLE_CONTROLLER: &str = "869e50c7";

pub fn build_disable_controller() -> String {
    format!("0x{}", SEL_DISABLE_CONTROLLER)
}

// ─── EVC (Euler Vault Connector) ───────────────────────────────────────────────

/// `enableCollateral(address account, address vault)` selector = 0xd44fee5a.
const SEL_ENABLE_COLLATERAL: &str = "d44fee5a";

pub fn build_enable_collateral(account: &str, vault: &str) -> String {
    format!(
        "0x{}{}{}",
        SEL_ENABLE_COLLATERAL, pad_address(account), pad_address(vault)
    )
}

/// `disableCollateral(address account, address vault)` selector = 0xe920e8e0.
const SEL_DISABLE_COLLATERAL: &str = "e920e8e0";

pub fn build_disable_collateral(account: &str, vault: &str) -> String {
    format!(
        "0x{}{}{}",
        SEL_DISABLE_COLLATERAL, pad_address(account), pad_address(vault)
    )
}

/// `enableController(address account, address vault)` selector = 0xc368516c.
const SEL_ENABLE_CONTROLLER: &str = "c368516c";

pub fn build_enable_controller(account: &str, vault: &str) -> String {
    format!(
        "0x{}{}{}",
        SEL_ENABLE_CONTROLLER, pad_address(account), pad_address(vault)
    )
}

/// EVC `getCollaterals(address account)` selector = 0xa4d25d1e — read helper.
pub const SEL_GET_COLLATERALS: &str = "a4d25d1e";

/// EVC `getControllers(address account)` selector = 0xfd6046d7 — read helper.
pub const SEL_GET_CONTROLLERS: &str = "fd6046d7";

#[cfg(test)]
mod tests {
    use super::*;
    use sha3::{Digest, Keccak256};

    /// Sanity-check the function selectors against keccak256 at runtime, so a
    /// bad copy/paste of a hardcoded hex string would fail the test instead of
    /// silently misrouting calls on-chain.
    fn sel(sig: &str) -> String {
        let h = Keccak256::digest(sig.as_bytes());
        hex::encode(&h[..4])
    }

    #[test]
    fn selectors_match_signatures() {
        assert_eq!(sel("approve(address,uint256)"), SEL_APPROVE);
        assert_eq!(sel("deposit(uint256,address)"), SEL_DEPOSIT);
        assert_eq!(sel("withdraw(uint256,address,address)"), SEL_WITHDRAW);
        assert_eq!(sel("redeem(uint256,address,address)"), SEL_REDEEM);
        assert_eq!(sel("borrow(uint256,address)"), SEL_BORROW);
        assert_eq!(sel("repay(uint256,address)"), SEL_REPAY);
        assert_eq!(sel("disableController()"), SEL_DISABLE_CONTROLLER);
        assert_eq!(sel("enableCollateral(address,address)"), SEL_ENABLE_COLLATERAL);
        assert_eq!(sel("disableCollateral(address,address)"), SEL_DISABLE_COLLATERAL);
        assert_eq!(sel("enableController(address,address)"), SEL_ENABLE_CONTROLLER);
        assert_eq!(sel("getCollaterals(address)"), SEL_GET_COLLATERALS);
        assert_eq!(sel("getControllers(address)"), SEL_GET_CONTROLLERS);
        assert_eq!(sel("skim(uint256,address)"), SEL_SKIM);
        assert_eq!(sel("transfer(address,uint256)"), SEL_TRANSFER);
        assert_eq!(sel("repayWithShares(uint256,address)"), SEL_REPAY_WITH_SHARES);
    }

    #[test]
    fn approve_calldata_shape() {
        let cd = build_approve("0x1111111111111111111111111111111111111111", 1_000_000);
        // 0x + 8 selector + 64 spender + 64 amount = 138 chars
        assert_eq!(cd.len(), 138);
        assert!(cd.starts_with("0x095ea7b3"));
    }

    #[test]
    fn repay_all_uses_max_uint() {
        let cd = build_repay_with_shares_all("0x1111111111111111111111111111111111111111");
        assert!(cd.contains(MAX_UINT256_HEX));
        assert!(cd.starts_with("0xa9c8eb7e"), "should use repayWithShares selector");
    }
}
